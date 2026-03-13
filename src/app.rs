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
    config::ConfigLoad,
    event,
    fs_scan::{self, ScanProgress},
    size::normalize_path,
    state::AppState,
    theme::Theme,
};
use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::tty::IsTty;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::runtime::Runtime;

pub fn run(cli_args: CliArgs, config_load: ConfigLoad) -> Result<()> {
    let ConfigLoad { config, error, .. } = config_load;

    if let Some(err) = error {
        eprintln!("config: {err}");
    }

    let root = normalize_path(cli_args.root);
    let mut exclude_patterns = config.scan.exclude_patterns.clone();
    exclude_patterns.extend(cli_args.exclude_patterns.clone());
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;

    let runtime = Runtime::new()?;
    let loop_result = runtime.block_on(async {
        let (scanner_handle, mut scanner_rx) = fs_scan::start_scan(
            root.clone(),
            config.scan.follow_symlinks,
            exclude_patterns,
            config.scan.count_hard_links_once,
        );
        let mut state = AppState::new(root.clone(), config.sorting.mode);
        state.mark_scan_progress(ScanProgress {
            scanned: 0,
            errors: 0,
        });
        state.update_status(format!("scanning {}", root.display()));

        let theme = Theme::default();
        event::run_event_loop(
            &mut terminal,
            &mut state,
            &mut scanner_rx,
            &scanner_handle,
            theme,
        )?;
        Ok::<(), anyhow::Error>(())
    });

    terminal.show_cursor()?;
    drop(terminal);
    drop(guard);

    loop_result?;
    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        let stdin_tty = io::stdin().is_tty();
        let stdout_tty = io::stdout().is_tty();

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen).map_err(|err| {
            anyhow::anyhow!(
                "failed to enter alternate screen (stdin_tty={stdin_tty}, stdout_tty={stdout_tty}): {err}"
            )
        })?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
