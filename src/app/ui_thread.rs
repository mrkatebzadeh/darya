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

use crate::events::run_event_loop;
use crate::scan::control::{ScanEventReceiver, ScanTriggerSender};
use crate::state::AppState;
use anyhow::{Result, anyhow};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::tty::IsTty;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::thread;
use tokio::sync::oneshot;

pub(crate) async fn run_ui_thread(
    state: AppState,
    scanner_rx: ScanEventReceiver,
    scan_trigger: ScanTriggerSender,
    theme: crate::theme::Theme,
) -> Result<()> {
    let (done_tx, done_rx) = oneshot::channel();
    thread::spawn(move || {
        let result = run_ui_loop(state, scanner_rx, scan_trigger, theme);
        let _ = done_tx.send(result);
    });
    done_rx
        .await
        .map_err(|_| anyhow!("UI thread was aborted"))??;
    Ok(())
}

fn run_ui_loop(
    mut state: AppState,
    mut scanner_rx: ScanEventReceiver,
    scan_trigger: ScanTriggerSender,
    theme: crate::theme::Theme,
) -> Result<()> {
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;
    let result = run_event_loop(
        &mut terminal,
        &mut state,
        &mut scanner_rx,
        &scan_trigger,
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
            "failed to enter alternate screen (stdin_tty={stdin_tty}, stdout_tty={stdout_tty}): {err}"
        ))?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
