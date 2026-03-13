// Copyright (C) 2026 M.R. Siavash Katebzadeh <mr@katebzadeh.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    cli::CliArgs,
    config::{Config, ConfigLoad},
    event,
    fs_scan::{self, ScanEvent, ScanOptions, ScanProgress, ScannerHandle, dummy_scanner},
    size::normalize_path,
    snapshot::{self, SnapshotEndpoint, SnapshotFormat},
    state::{AppState, ScanState},
    theme::Theme,
};
use anyhow::{Result, anyhow};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::tty::IsTty;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::path::PathBuf;
use std::thread;
use tokio::runtime::Builder;
use tokio::sync::{mpsc::UnboundedReceiver, oneshot};

pub fn run(cli_args: CliArgs, config_load: ConfigLoad) -> Result<()> {
    let ConfigLoad { config, error, .. } = config_load;
    if let Some(err) = error {
        eprintln!("config: {err}");
    }

    let root = normalize_path(cli_args.root.clone());
    let mut exclude_patterns = config.scan.exclude_patterns.clone();
    exclude_patterns.extend(cli_args.exclude_patterns.iter().cloned());

    let follow_symlinks = cli_args
        .follow_symlinks_override
        .unwrap_or(config.scan.follow_symlinks);
    let same_file_system = cli_args
        .same_fs_override
        .unwrap_or(config.scan.one_file_system);
    let skip_caches = match cli_args.cache_policy {
        Some(true) => false,
        Some(false) => true,
        None => config.scan.exclude_caches,
    };
    let skip_kernfs = match cli_args.kernfs_policy {
        Some(true) => false,
        Some(false) => true,
        None => config.scan.exclude_kernfs,
    };

    let scan_options = ScanOptions {
        follow_symlinks,
        count_hard_links_once: config.scan.count_hard_links_once,
        same_file_system,
        skip_caches,
        skip_kernfs,
    };

    let theme = Theme::default();
    let thread_count = cli_args.thread_count.or(config.scan.thread_count);
    let mut builder = Builder::new_multi_thread();
    if let Some(threads) = thread_count
        && threads > 0
    {
        builder.worker_threads(threads);
    }
    let runtime = builder.enable_all().build()?;
    runtime.block_on(async move {
        if let Some(import_endpoint) = cli_args.import_snapshot.clone() {
            run_import_mode(
                root.clone(),
                config.sorting.mode,
                import_endpoint,
                theme,
                cli_args.extended,
            )
            .await?;
        } else if cli_args.export_json.is_some() || cli_args.export_binary.is_some() {
            run_export_mode(
                root.clone(),
                exclude_patterns.clone(),
                &config,
                cli_args.export_json.clone(),
                cli_args.export_binary.clone(),
                cli_args.extended,
                scan_options,
            )
            .await?;
        } else {
            run_interactive_mode(
                root.clone(),
                exclude_patterns,
                &config,
                theme,
                cli_args.extended,
                scan_options,
            )
            .await?;
        }
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

async fn run_interactive_mode(
    root: PathBuf,
    exclude_patterns: Vec<String>,
    config: &Config,
    theme: Theme,
    extended: bool,
    scan_options: ScanOptions,
) -> Result<()> {
    let (scanner_handle, scanner_rx) =
        fs_scan::start_scan(root.clone(), scan_options, exclude_patterns);
    let mut state = AppState::new(root.clone(), config.sorting.mode);
    state.set_extended_mode(extended);
    state.mark_scan_progress(ScanProgress {
        scanned: 0,
        errors: 0,
    });
    state.update_status(format!("scanning {}", root.display()));
    run_ui_thread(state, scanner_rx, scanner_handle, theme).await
}

async fn run_import_mode(
    root: PathBuf,
    sort_mode: crate::config::SortMode,
    endpoint: SnapshotEndpoint,
    theme: Theme,
    extended: bool,
) -> Result<()> {
    let mut state = AppState::new(root.clone(), sort_mode);
    state.set_extended_mode(extended);
    let default_root = state
        .tree
        .node(state.tree.root())
        .map(|node| node.path.clone())
        .unwrap_or(root.clone());
    state.tree = snapshot::import_from_destination(endpoint, &default_root, SnapshotFormat::Json)?;
    state.selection = Some(state.tree.root());
    state.allow_modifications = false;
    let (scanner_handle, scanner_rx) = dummy_scanner();
    run_ui_thread(state, scanner_rx, scanner_handle, theme).await
}

async fn run_export_mode(
    root: PathBuf,
    exclude_patterns: Vec<String>,
    config: &Config,
    export_json: Option<SnapshotEndpoint>,
    export_binary: Option<SnapshotEndpoint>,
    extended: bool,
    scan_options: ScanOptions,
) -> Result<()> {
    let (scanner_handle, mut scanner_rx) =
        fs_scan::start_scan(root.clone(), scan_options, exclude_patterns);
    let mut state = AppState::new(root, config.sorting.mode);
    state.set_extended_mode(extended);
    state.mark_scan_progress(ScanProgress {
        scanned: 0,
        errors: 0,
    });
    while let Some(event) = scanner_rx.recv().await {
        event::process_scan_event(&mut state, event);
        if matches!(state.scan_state, ScanState::Completed) {
            break;
        }
    }
    if let Some(dest) = export_json {
        snapshot::export_to_destination(&state.tree, dest, SnapshotFormat::Json)?;
    }
    if let Some(dest) = export_binary {
        snapshot::export_to_destination(&state.tree, dest, SnapshotFormat::Binary)?;
    }
    scanner_handle.cancel();
    Ok(())
}

async fn run_ui_thread(
    state: AppState,
    scanner_rx: UnboundedReceiver<ScanEvent>,
    scanner_handle: ScannerHandle,
    theme: Theme,
) -> Result<()> {
    let (done_tx, done_rx) = oneshot::channel();
    thread::spawn(move || {
        let result = run_ui_loop(state, scanner_rx, scanner_handle, theme);
        let _ = done_tx.send(result);
    });
    done_rx
        .await
        .map_err(|_| anyhow!("UI thread was aborted"))??;
    Ok(())
}

fn run_ui_loop(
    mut state: AppState,
    mut scanner_rx: UnboundedReceiver<ScanEvent>,
    scanner_handle: ScannerHandle,
    theme: Theme,
) -> Result<()> {
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;
    let result = event::run_event_loop(
        &mut terminal,
        &mut state,
        &mut scanner_rx,
        &scanner_handle,
        theme,
    );
    terminal.show_cursor()?;
    drop(terminal);
    drop(guard);
    result
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        let stdin_tty = io::stdin().is_tty();
        let stdout_tty = io::stdout().is_tty();

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen).map_err(|err| anyhow!(
            "failed to enter alternate screen (stdin_tty={stdin_tty}, stdout_tty={stdout_tty}): {err}"))?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
