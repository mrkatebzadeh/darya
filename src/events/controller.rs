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

use crate::config::SortMode;
use crate::events::actions::{files, filter, navigation, scan};
use crate::fs_scan::{ScanEvent, ScanProgress};
use crate::input::InputAction;
use crate::scan_control::{ScanTrigger, ScanTriggerSender};
use crate::state::{AppState, ScanState, StatusMessage};

pub fn handle_input_action(
    action: InputAction,
    state: &mut AppState,
    scan_trigger: &ScanTriggerSender,
) {
    match action {
        InputAction::MoveUp => navigation::select_previous(state),
        InputAction::MoveDown => navigation::select_next(state),
        InputAction::JumpTop => navigation::select_first(state),
        InputAction::JumpBottom => navigation::select_last(state),
        InputAction::Expand => navigation::expand_selection(state),
        InputAction::Select => navigation::toggle_selection(state),
        InputAction::Delete => files::delete_selection(state),
        InputAction::Open => files::open_selection(state),
        InputAction::ToggleSizeMode => state.toggle_size_mode(),
        InputAction::ToggleTreemap => state.toggle_treemap_visibility(),
        InputAction::ExportScan => files::export_scan(state),
        InputAction::ImportScan => files::import_scan(state),
        InputAction::Rescan => scan::rescan_selection(state),
        InputAction::StartFilter => {
            filter::start_filter(state);
        }
        InputAction::FilterChar(ch) => filter::filter_char(state, ch),
        InputAction::FilterBackspace => filter::filter_backspace(state),
        InputAction::ApplyFilter => filter::apply_filter(state),
        InputAction::ClearFilter => filter::clear_filter(state),
        InputAction::CycleSort => {
            let next = next_sort_mode(state.sort_mode);
            state.set_sort_mode(next);
            state.update_status(StatusMessage::SortMode(next));
        }
        InputAction::ToggleHidden => {
            state.display_options.show_hidden = !state.display_options.show_hidden;
            state.refresh_treemap_nodes();
            state.ensure_selection_visible();
            state.update_status(StatusMessage::HiddenFilesVisible(
                state.display_options.show_hidden,
            ));
        }
        InputAction::ToggleHelp => {
            state.show_help = !state.show_help;
            if state.show_help {
                state.update_status(StatusMessage::HelpOpened);
            } else {
                state.update_status(StatusMessage::HelpClosed);
            }
        }
        InputAction::Collapse => navigation::collapse_selection(state),
        InputAction::StartScan => {
            if !matches!(state.scan_state, ScanState::Running(_)) {
                let _ = scan_trigger.send(ScanTrigger::Start);
                state.mark_scan_progress(ScanProgress {
                    scanned: 0,
                    errors: 0,
                });
            }
        }
        _ => {}
    }

    state.refresh_ui();
}

fn next_sort_mode(current: SortMode) -> SortMode {
    match current {
        SortMode::SizeDesc => SortMode::SizeAsc,
        SortMode::SizeAsc => SortMode::Name,
        SortMode::Name => SortMode::ModifiedTime,
        SortMode::ModifiedTime => SortMode::SizeDesc,
    }
}

pub fn process_scan_event(state: &mut AppState, event: ScanEvent) {
    scan::process_scan_event(state, event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_scan::ScanActivity;
    use crate::fs_scan::ScanBatch;
    use crate::fs_scan::ScanNode;
    use crate::scan_control::ScanTriggerSender;
    use crate::state::AppState;
    use crate::tree::{NodeType, TreeNode};
    use std::path::{Path, PathBuf};
    use tokio::sync::mpsc;

    fn sample_state() -> AppState {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let child = TreeNode::new(PathBuf::from("/child"), NodeType::File);
        state.tree.add_child(0, child);
        state.navigation.selection = Some(0);
        state
    }

    fn default_sort_mode() -> crate::config::SortMode {
        crate::config::SortMode::SizeDesc
    }

    fn dummy_trigger() -> ScanTriggerSender {
        let (tx, _rx) = mpsc::unbounded_channel();
        tx
    }

    #[test]
    fn move_down_moves_selection() {
        let mut state = sample_state();
        let trigger = dummy_trigger();
        handle_input_action(InputAction::MoveDown, &mut state, &trigger);
        assert_eq!(state.navigation.selection, Some(1));
    }

    #[test]
    fn move_up_wraps_to_root() {
        let mut state = sample_state();
        state.navigation.selection = Some(1);
        let trigger = dummy_trigger();
        handle_input_action(InputAction::MoveUp, &mut state, &trigger);
        assert_eq!(state.navigation.selection, Some(0));
    }

    #[test]
    fn select_toggles_directory_expansion() {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let dir_id = state.tree.add_child(
            0,
            TreeNode::new(PathBuf::from("/dir"), NodeType::Directory).collapsed(),
        );
        state.navigation.selection = Some(dir_id);
        let trigger = dummy_trigger();

        handle_input_action(InputAction::Select, &mut state, &trigger);
        assert!(state.tree.node(dir_id).unwrap().expanded);

        handle_input_action(InputAction::Select, &mut state, &trigger);
        assert!(!state.tree.node(dir_id).unwrap().expanded);
    }

    #[test]
    fn delete_action_first_press_requests_confirmation() {
        let mut state = sample_state();
        state.navigation.selection = Some(1);
        let trigger = dummy_trigger();
        handle_input_action(InputAction::Delete, &mut state, &trigger);
        assert_eq!(state.pending_delete, Some(1));
    }

    #[test]
    fn open_command_matches_platform() {
        let command = crate::events::actions::files::open_command_for_platform();
        let expected = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        assert_eq!(command.get_program().to_string_lossy(), expected);
    }

    #[test]
    fn toggle_treemap_updates_visibility() {
        let mut state = sample_state();
        let trigger = dummy_trigger();
        let initial = state.treemap_visible;

        handle_input_action(InputAction::ToggleTreemap, &mut state, &trigger);

        assert_ne!(state.treemap_visible, initial);
        let expected = if state.treemap_visible {
            "treemap panel shown"
        } else {
            "treemap panel hidden"
        };
        assert_eq!(state.status_message.unwrap().to_string(), expected);
    }

    #[test]
    fn adjust_ancestors_updates_parent_sizes() {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let dir_id = state.tree.add_child(
            0,
            TreeNode::new(PathBuf::from("/dir"), NodeType::Directory).with_size(10),
        );
        if let Some(root) = state.tree.node_mut(0) {
            root.size = 10;
        }
        crate::events::actions::scan::adjust_ancestors_after_rescan(&mut state, Some(0), 10, 25);
        assert_eq!(state.tree.node(0).unwrap().size, 25);
        assert_eq!(dir_id, 1);
    }

    #[test]
    fn next_sort_mode_cycles() {
        assert_eq!(next_sort_mode(SortMode::SizeDesc), SortMode::SizeAsc);
        assert_eq!(next_sort_mode(SortMode::SizeAsc), SortMode::Name);
        assert_eq!(next_sort_mode(SortMode::Name), SortMode::ModifiedTime);
        assert_eq!(next_sort_mode(SortMode::ModifiedTime), SortMode::SizeDesc);
    }

    #[test]
    fn batch_updates_root_size() {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let nodes = vec![
            ScanNode {
                path: PathBuf::from("/a"),
                kind: NodeType::File,
                size: 2048,
                disk_size: 2048,
                modified: None,
                permissions: None,
                uid: None,
                gid: None,
            },
            ScanNode {
                path: PathBuf::from("/dir/b"),
                kind: NodeType::File,
                size: 4096,
                disk_size: 4096,
                modified: None,
                permissions: None,
                uid: None,
                gid: None,
            },
        ];
        let batch = ScanBatch {
            nodes,
            progress: None,
            activity: Some(ScanActivity::default()),
        };
        process_scan_event(&mut state, ScanEvent::Batch(batch));
        if matches!(state.scan_state, ScanState::Completed) {
            process_scan_event(&mut state, ScanEvent::Completed);
        }
        let root = state.tree.node(0).unwrap();
        assert_eq!(root.size, 2048 + 4096);
        assert!(state.tree.verify_size_invariants());
    }

    #[test]
    fn batch_preserves_parent_child_sum() {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let nodes = vec![
            ScanNode {
                path: PathBuf::from("/dir/a"),
                kind: NodeType::File,
                size: 1024,
                disk_size: 1024,
                modified: None,
                permissions: None,
                uid: None,
                gid: None,
            },
            ScanNode {
                path: PathBuf::from("/dir/b"),
                kind: NodeType::File,
                size: 2048,
                disk_size: 2048,
                modified: None,
                permissions: None,
                uid: None,
                gid: None,
            },
        ];
        let batch = ScanBatch {
            nodes,
            progress: Some(ScanProgress {
                scanned: 2,
                errors: 0,
            }),
            activity: Some(ScanActivity::default()),
        };
        process_scan_event(&mut state, ScanEvent::Batch(batch));
        process_scan_event(&mut state, ScanEvent::Completed);
        assert!(state.tree.verify_size_invariants());
        let parent = state.tree.node(0).unwrap().children[0];
        assert_eq!(state.tree.node(parent).unwrap().size, 3072);
    }

    #[test]
    fn multi_level_batch_sums() {
        let mut state = AppState::new(PathBuf::from("/"), default_sort_mode());
        let nodes = vec![
            ScanNode {
                path: PathBuf::from("/dir/sub/file1"),
                kind: NodeType::File,
                size: 4096,
                disk_size: 4096,
                modified: None,
                permissions: None,
                uid: None,
                gid: None,
            },
            ScanNode {
                path: PathBuf::from("/dir/sub/file2"),
                kind: NodeType::File,
                size: 2048,
                disk_size: 2048,
                modified: None,
                permissions: None,
                uid: None,
                gid: None,
            },
        ];
        let batch = ScanBatch {
            nodes,
            progress: None,
            activity: Some(ScanActivity::default()),
        };
        process_scan_event(&mut state, ScanEvent::Batch(batch));
        process_scan_event(&mut state, ScanEvent::Completed);
        assert!(state.tree.verify_size_invariants());
        let dir = state.tree.node_id_for_path(Path::new("/dir")).unwrap();
        let sub = state.tree.node_id_for_path(Path::new("/dir/sub")).unwrap();
        assert_eq!(state.tree.node(sub).unwrap().size, 6144);
        assert_eq!(state.tree.node(dir).unwrap().size, 6144);
    }
}
