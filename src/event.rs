use crate::fs_scan::{ScanEvent, ScannerHandle};
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
    input::{InputAction, InputState},
    layout,
    size::normalize_path,
    state::AppState,
    theme::Theme,
    tree::NodeType,
    ui::Ui,
};
use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::CrosstermBackend, terminal::Terminal};
use std::fs;
use std::{
    io::Stdout,
    process::Command,
    time::{Duration, Instant},
};
use throbber_widgets_tui::BRAILLE_EIGHT;
use tokio::sync::mpsc::UnboundedReceiver;

const TICK_RATE: Duration = Duration::from_millis(250);
const MAX_SCAN_EVENTS_PER_CYCLE: usize = 256;

/// Runs the terminal event loop until the user quits or scanning completes.
pub fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut AppState,
    scanner_rx: &mut UnboundedReceiver<ScanEvent>,
    scanner_handle: &ScannerHandle,
    theme: Theme,
) -> Result<()> {
    let mut input_state = InputState::new();
    let ui = Ui;
    let mut last_tick = Instant::now();
    let mut dirty = true;
    let mut should_quit = false;

    if state.selection.is_none() {
        state.selection = Some(state.tree.root());
    }

    while !should_quit {
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key_event) => {
                    let action = input_state.process_key(key_event);
                    if matches!(action, InputAction::Quit) {
                        should_quit = true;
                        scanner_handle.cancel();
                    }
                    handle_input_action(action, state);
                    dirty = true;
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }

        for _ in 0..MAX_SCAN_EVENTS_PER_CYCLE {
            match scanner_rx.try_recv() {
                Ok(scan_event) => {
                    handle_scan_event(state, scan_event);
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

fn handle_input_action(action: InputAction, state: &mut AppState) {
    match action {
        InputAction::MoveUp => select_previous(state),
        InputAction::MoveDown => select_next(state),
        InputAction::JumpTop => select_first(state),
        InputAction::JumpBottom => select_last(state),
        InputAction::Expand => expand_selection(state),
        InputAction::Select => toggle_selection(state),
        InputAction::Delete => delete_selection(state),
        InputAction::Open => open_selection(state),
        InputAction::Collapse => collapse_selection(state),
        _ => {}
    }
}

fn select_previous(state: &mut AppState) {
    let ids = state.tree.visible_ids();
    if ids.is_empty() {
        state.selection = None;
        return;
    }
    let index = ids
        .iter()
        .position(|&id| Some(id) == state.selection)
        .unwrap_or(0);
    let next = ids.get(index.saturating_sub(1)).copied().unwrap_or(ids[0]);
    state.selection = Some(next);
}

fn select_next(state: &mut AppState) {
    let ids = state.tree.visible_ids();
    if ids.is_empty() {
        state.selection = None;
        return;
    }
    let index = ids
        .iter()
        .position(|&id| Some(id) == state.selection)
        .unwrap_or(usize::MAX);
    let next = if index + 1 >= ids.len() {
        ids[ids.len() - 1]
    } else {
        ids[index + 1]
    };
    state.selection = Some(next);
}

fn select_first(state: &mut AppState) {
    let ids = state.tree.visible_ids();
    if let Some(&first) = ids.first() {
        state.selection = Some(first);
    }
}

fn select_last(state: &mut AppState) {
    let ids = state.tree.visible_ids();
    if let Some(&last) = ids.last() {
        state.selection = Some(last);
    }
}

fn expand_selection(state: &mut AppState) {
    if let Some(id) = state.selection
        && let Some(node) = state.tree.node_mut(id)
        && node.file_type == NodeType::Directory
    {
        node.expanded = true;
    }
}

fn collapse_selection(state: &mut AppState) {
    if let Some(id) = state.selection
        && let Some(node) = state.tree.node_mut(id)
    {
        if node.file_type == NodeType::Directory {
            node.expanded = false;
            return;
        }
        if let Some(parent) = node.parent
            && let Some(parent_node) = state.tree.node_mut(parent)
        {
            parent_node.expanded = false;
            state.selection = Some(parent);
        }
    }
}

fn toggle_selection(state: &mut AppState) {
    if let Some(id) = state.selection
        && let Some(node) = state.tree.node_mut(id)
        && node.file_type == NodeType::Directory
    {
        node.expanded = !node.expanded;
    }
}

fn delete_selection(state: &mut AppState) {
    let Some(selected_id) = state.selection else {
        return;
    };

    if state.pending_delete != Some(selected_id) {
        state.pending_delete = Some(selected_id);
        if let Some(node) = state.tree.node(selected_id) {
            state.update_status(format!("press d again to delete {}", node.path.display()));
        }
        return;
    }

    let Some(target) = state.tree.node(selected_id).map(|n| n.path.clone()) else {
        state.pending_delete = None;
        return;
    };

    let result = if target.is_dir() {
        fs::remove_dir_all(&target)
    } else {
        fs::remove_file(&target)
    };

    match result {
        Ok(()) => {
            if let Some(node) = state.tree.node_mut(selected_id) {
                if !node.name.starts_with("✖ ") {
                    node.name = format!("✖ {}", node.name);
                }
                node.size = 0;
                node.children.clear();
                node.expanded = false;
            }
            state.update_status(format!("deleted {}", target.display()));
        }
        Err(err) => {
            state.mark_scan_error(format!("delete failed for {}: {err}", target.display()));
            state.update_status(format!("delete failed: {}", target.display()));
        }
    }
    state.pending_delete = None;
}

fn open_selection(state: &mut AppState) {
    let Some(selected_id) = state.selection else {
        return;
    };
    let Some(path) = state.tree.node(selected_id).map(|n| n.path.clone()) else {
        return;
    };

    let mut command = open_command_for_platform();
    command.arg(&path);

    match command.spawn() {
        Ok(_) => state.update_status(format!("opened {}", path.display())),
        Err(err) => state.update_status(format!("open failed for {}: {err}", path.display())),
    }
}

fn open_command_for_platform() -> Command {
    if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    }
}

fn handle_scan_event(state: &mut AppState, event: ScanEvent) {
    match event {
        ScanEvent::Node(node) => {
            let normalized = normalize_path(&node.path);
            let node_id = state.tree.ensure_node(normalized.clone(), node.kind);
            if node.kind == NodeType::File {
                state.tree.add_size(node_id, node.size);
            }
            if let Some(parent) = state.tree.node(node_id).and_then(|node| node.parent) {
                state.tree.sort_children(parent, state.sort_mode);
            }
            state.update_status(format!("scanned {}", normalized.display()));
        }
        ScanEvent::Progress(progress) => {
            state.mark_scan_progress(progress.clone());
            state.update_status(format!(
                "scanned {} entries, {} errors",
                progress.scanned, progress.errors
            ));
        }
        ScanEvent::Error(error) => {
            state.mark_scan_error(format!("{}: {}", error.path.display(), error.source));
        }
        ScanEvent::Completed => {
            state.mark_scan_complete();
            state.update_status("scan complete");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use crate::tree::{NodeType, TreeNode};
    use std::path::PathBuf;

    fn sample_state() -> AppState {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let child = TreeNode::new(PathBuf::from("/child"), NodeType::File);
        state.tree.add_child(0, child);
        state.selection = Some(0);
        state
    }

    fn default_sort_mode() -> crate::config::SortMode {
        crate::config::SortMode::SizeDesc
    }

    #[test]
    fn move_down_moves_selection() {
        let mut state = sample_state();
        handle_input_action(InputAction::MoveDown, &mut state);
        assert_eq!(state.selection, Some(1));
    }

    #[test]
    fn move_up_wraps_to_root() {
        let mut state = sample_state();
        state.selection = Some(1);
        handle_input_action(InputAction::MoveUp, &mut state);
        assert_eq!(state.selection, Some(0));
    }

    #[test]
    fn select_toggles_directory_expansion() {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let dir_id = state.tree.add_child(
            0,
            TreeNode::new(PathBuf::from("/dir"), NodeType::Directory).collapsed(),
        );
        state.selection = Some(dir_id);

        handle_input_action(InputAction::Select, &mut state);
        assert!(state.tree.node(dir_id).unwrap().expanded);

        handle_input_action(InputAction::Select, &mut state);
        assert!(!state.tree.node(dir_id).unwrap().expanded);
    }

    #[test]
    fn delete_action_first_press_requests_confirmation() {
        let mut state = sample_state();
        state.selection = Some(1);
        handle_input_action(InputAction::Delete, &mut state);
        assert_eq!(state.pending_delete, Some(1));
    }

    #[test]
    fn open_command_matches_platform() {
        let command = open_command_for_platform();
        let expected = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        assert_eq!(command.get_program().to_string_lossy(), expected);
    }
}
