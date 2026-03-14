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
    cli::{CliArgs, InterfaceMode},
    config::{Config, ConfigLoad},
    fs_scan::ScanOptions,
    scan_control::{ScanEventReceiver, ScanEventSender, ScanTriggerReceiver, ScanTriggerSender},
    size::normalize_path,
    snapshot::ExportOptions,
    state::AppState,
    theme::Theme,
};
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use tokio::runtime::Builder;

mod modes;
mod scan_manager;
mod ui_thread;

use modes::{
    export_options_from_cli, run_export_mode, run_import_mode, run_progress_mode, run_summary_mode,
};
use scan_manager::scan_manager;
use ui_thread::run_ui_thread;

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
    let export_options = export_options_from_cli(&cli_args);
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
                export_options,
            )
            .await?;
        } else {
            match cli_args.interface_mode {
                InterfaceMode::Tui => {
                    run_interactive_mode(
                        root.clone(),
                        exclude_patterns.clone(),
                        &config,
                        theme,
                        cli_args.extended,
                        scan_options,
                        export_options,
                    )
                    .await?;
                }
                InterfaceMode::Progress => {
                    run_progress_mode(root.clone(), exclude_patterns.clone(), scan_options).await?;
                }
                InterfaceMode::Summary => {
                    run_summary_mode(root.clone(), exclude_patterns, scan_options).await?;
                }
            }
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
    export_options: ExportOptions,
) -> Result<()> {
    let (scan_event_tx, scan_event_rx): (ScanEventSender, ScanEventReceiver) =
        tokio::sync::mpsc::unbounded_channel();
    let (scan_trigger_tx, scan_trigger_rx): (ScanTriggerSender, ScanTriggerReceiver) =
        tokio::sync::mpsc::unbounded_channel();
    let manager = tokio::spawn(scan_manager(
        scan_trigger_rx,
        scan_event_tx.clone(),
        root.clone(),
        exclude_patterns.clone(),
        scan_options,
    ));
    let mut state = AppState::new(root.clone(), config.sorting.mode);
    state.set_extended_mode(extended);
    state.set_export_options(export_options);
    state.update_status(format!("press R to scan {}", root.display()));
    run_ui_thread(state, scan_event_rx, scan_trigger_tx.clone(), theme).await?;
    drop(scan_trigger_tx);
    manager
        .await
        .map_err(|err| anyhow!("scan manager aborted: {err}"))?;
    Ok(())
}
