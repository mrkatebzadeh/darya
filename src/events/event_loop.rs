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

use crate::events::handlers::{handle_input_action, process_scan_event};
use crate::fs_scan::ScanEvent;
use crate::input::{InputAction, InputState};
use crate::scan_control::ScanTriggerSender;
use crate::state::AppState;
use crate::theme::Theme;
use crate::ui::{Ui, layout};
use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, terminal::Terminal};
use std::io::Stdout;
use std::time::{Duration, Instant};
use throbber_widgets_tui::BRAILLE_EIGHT;
use tokio::sync::mpsc::UnboundedReceiver;

const TICK_RATE: Duration = Duration::from_millis(250);
const MAX_SCAN_EVENTS_PER_CYCLE: usize = 256;

/// Runs the terminal event loop until the user quits.
pub fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut AppState,
    scanner_rx: &mut UnboundedReceiver<ScanEvent>,
    scan_trigger: &ScanTriggerSender,
    theme: Theme,
) -> Result<()> {
    let mut input_state = InputState::new();
    let mut ui = Ui::default();
    let mut last_tick = Instant::now();
    let mut dirty = true;
    let mut should_quit = false;

    if state.selection.is_none() {
        state.selection = Some(state.tree.root());
    }
    state.refresh_treemap_nodes();

    while !should_quit {
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) => {
                    let action = input_state.process_key(key_event);
                    if matches!(action, InputAction::Quit) {
                        should_quit = true;
                        let _ = scan_trigger.send(crate::scan_control::ScanTrigger::Cancel);
                    }
                    handle_input_action(action, state, scan_trigger);
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }

        for _ in 0..MAX_SCAN_EVENTS_PER_CYCLE {
            match scanner_rx.try_recv() {
                Ok(scan_event) => {
                    process_scan_event(state, scan_event);
                    dirty = true;
                }
                Err(_) => break,
            }
        }

        if last_tick.elapsed() >= TICK_RATE || dirty {
            state.advance_spinner(BRAILLE_EIGHT.symbols.len());
            terminal.draw(|frame| {
                let regions = layout::split_layout(frame.size());
                ui.draw(frame, regions, state, theme);
            })?;
            last_tick = Instant::now();
            dirty = false;
        }
    }

    Ok(())
}
