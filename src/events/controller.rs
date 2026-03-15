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
use crate::fs_scan::{ScanEvent, ScanProgress};
use crate::input::InputAction;
use crate::scan_control::{ScanTrigger, ScanTriggerSender};
use crate::size::{normalize_path, total_size};
use crate::snapshot::{self, SnapshotEndpoint, SnapshotFormat};
use crate::state::{AppState, ScanState};
use crate::tree::{NodeMetadata, NodeType};
use std::fs;
use std::process::Command;

pub fn handle_input_action(
    action: InputAction,
    state: &mut AppState,
    scan_trigger: &ScanTriggerSender,
) {
    match action {
        InputAction::MoveUp => select_previous(state),
        InputAction::MoveDown => select_next(state),
        InputAction::JumpTop => select_first(state),
        InputAction::JumpBottom => select_last(state),
        InputAction::Expand => expand_selection(state),
        InputAction::Select => toggle_selection(state),
        InputAction::Delete => delete_selection(state),
        InputAction::Open => open_selection(state),
        InputAction::ToggleSizeMode => state.toggle_size_mode(),
        InputAction::ExportScan => export_scan(state),
        InputAction::ImportScan => import_scan(state),
        InputAction::Rescan => rescan_selection(state),
        InputAction::StartFilter => {
            state.filter_active = true;
            state.filter_prompt_active = true;
            state.update_status("filter: type name substring and press Enter");
        }
        InputAction::FilterChar(ch) => {
            state.filter_query.push(ch);
            state.filter_active = true;
        }
        InputAction::FilterBackspace => {
            state.filter_query.pop();
        }
        InputAction::ApplyFilter => {
            state.filter_prompt_active = false;
            if state.filter_query.is_empty() {
                state.filter_active = false;
                state.update_status("filter cleared");
            } else {
                state.filter_active = true;
                state.update_status(format!("filter active: {}", state.filter_query));
            }
        }
        InputAction::ClearFilter => {
            state.filter_prompt_active = false;
            state.clear_filter();
            state.update_status("filter cleared");
        }
        InputAction::CycleSort => {
            let next = next_sort_mode(state.sort_mode);
            state.set_sort_mode(next);
            state.update_status(format!("sort mode: {}", sort_mode_label(next)));
        }
        InputAction::ToggleHidden => {
            state.display_options.show_hidden = !state.display_options.show_hidden;
            state.refresh_treemap_nodes();
            state.ensure_selection_visible();
            let status = if state.display_options.show_hidden {
                "hidden files shown"
            } else {
                "hidden files hidden"
            };
            state.update_status(status);
        }
        InputAction::ToggleHelp => {
            state.show_help = !state.show_help;
            if state.show_help {
                state.update_status("help opened");
            } else {
                state.update_status("help closed");
            }
        }
        InputAction::Collapse => collapse_selection(state),
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

    state.refresh_treemap_nodes();
}

fn next_sort_mode(current: SortMode) -> SortMode {
    match current {
        SortMode::SizeDesc => SortMode::SizeAsc,
        SortMode::SizeAsc => SortMode::Name,
        SortMode::Name => SortMode::ModifiedTime,
        SortMode::ModifiedTime => SortMode::SizeDesc,
    }
}

fn sort_mode_label(mode: SortMode) -> &'static str {
    match mode {
        SortMode::SizeDesc => "size_desc",
        SortMode::SizeAsc => "size_asc",
        SortMode::Name => "name",
        SortMode::ModifiedTime => "modified_time",
    }
}

fn select_previous(state: &mut AppState) {
    let ids = state.visible_node_ids();
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
    let ids = state.visible_node_ids();
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
    let ids = state.visible_node_ids();
    if let Some(&first) = ids.first() {
        state.selection = Some(first);
    }
}

fn select_last(state: &mut AppState) {
    let ids = state.visible_node_ids();
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
    if !state.allow_modifications {
        state.update_status("imported scan is read-only");
        return;
    }
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
    if !state.allow_modifications {
        state.update_status("imported scan is read-only");
        return;
    }
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

fn export_scan(state: &mut AppState) {
    let snapshot_path = std::path::Path::new("/tmp/dar-scan.json");
    match snapshot::export_tree(&state.tree, snapshot_path, state.export_options) {
        Ok(()) => state.update_status(format!("scan exported to {}", snapshot_path.display())),
        Err(err) => state.update_status(format!("export failed: {err}")),
    }
}

fn import_scan(state: &mut AppState) {
    let snapshot_path = std::path::Path::new("/tmp/dar-scan.json");
    let default_root = state
        .tree
        .node(state.tree.root())
        .map(|node| node.path.clone())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    match snapshot::import_from_destination(
        SnapshotEndpoint::File(snapshot_path.to_path_buf()),
        &default_root,
        SnapshotFormat::Json,
    ) {
        Ok(tree) => {
            state.tree = tree;
            state.selection = Some(state.tree.root());
            state.update_status(format!("scan imported from {}", snapshot_path.display()));
        }
        Err(err) => state.update_status(format!("import failed: {err}")),
    }
}

fn rescan_selection(state: &mut AppState) {
    if !state.allow_modifications {
        state.update_status("imported scan is read-only");
        return;
    }
    let Some(selected_id) = state.selection else {
        return;
    };
    let Some(node) = state.tree.node(selected_id).cloned() else {
        return;
    };

    let previous_size = node.size;
    let refreshed_size = if node.file_type == NodeType::Directory {
        match total_size(&node.path, false) {
            Ok(size) => size,
            Err(err) => {
                state.update_status(format!("rescan failed: {err}"));
                return;
            }
        }
    } else {
        match std::fs::metadata(&node.path) {
            Ok(meta) => meta.len(),
            Err(err) => {
                state.update_status(format!("rescan failed: {err}"));
                return;
            }
        }
    };

    if let Some(current) = state.tree.node_mut(selected_id) {
        current.size = refreshed_size;
    }
    adjust_ancestors_after_rescan(state, node.parent, previous_size, refreshed_size);
    state.update_status(format!("rescanned {}", node.path.display()));
}

fn adjust_ancestors_after_rescan(
    state: &mut AppState,
    mut parent: Option<usize>,
    previous_size: u64,
    refreshed_size: u64,
) {
    while let Some(parent_id) = parent {
        if let Some(parent_node) = state.tree.node_mut(parent_id) {
            if refreshed_size >= previous_size {
                parent_node.size = parent_node
                    .size
                    .saturating_add(refreshed_size.saturating_sub(previous_size));
            } else {
                parent_node.size = parent_node
                    .size
                    .saturating_sub(previous_size.saturating_sub(refreshed_size));
            }
            parent = parent_node.parent;
        } else {
            break;
        }
    }
}

fn open_command_for_platform() -> Command {
    if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    }
}

pub fn process_scan_event(state: &mut AppState, event: ScanEvent) {
    match event {
        ScanEvent::Node(node) => {
            let normalized = normalize_path(&node.path);
            let node_id = state.tree.ensure_node(normalized.clone(), node.kind);
            if node.kind == NodeType::File {
                state.tree.add_size(node_id, node.size);
                state.tree.add_disk_size(node_id, node.disk_size);
            }
            if state.extended_mode {
                state.tree.set_node_metadata(
                    node_id,
                    NodeMetadata {
                        modified: node.modified,
                        permissions: node.permissions,
                        uid: node.uid,
                        gid: node.gid,
                    },
                );
            }
            if let Some(parent) = state.tree.node(node_id).and_then(|node| node.parent) {
                state.tree.sort_children(parent, state.sort_mode);
            }
            state.update_status(format!("scanned {}", normalized.display()));
        }
        ScanEvent::Activity(activity) => {
            state.scan_activity = activity;
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

    state.refresh_treemap_nodes();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan_control::ScanTriggerSender;
    use crate::state::AppState;
    use crate::tree::{NodeType, TreeNode};
    use std::path::PathBuf;
    use tokio::sync::mpsc;

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

    fn dummy_trigger() -> ScanTriggerSender {
        let (tx, _rx) = mpsc::unbounded_channel();
        tx
    }

    #[test]
    fn move_down_moves_selection() {
        let mut state = sample_state();
        let trigger = dummy_trigger();
        handle_input_action(InputAction::MoveDown, &mut state, &trigger);
        assert_eq!(state.selection, Some(1));
    }

    #[test]
    fn move_up_wraps_to_root() {
        let mut state = sample_state();
        state.selection = Some(1);
        let trigger = dummy_trigger();
        handle_input_action(InputAction::MoveUp, &mut state, &trigger);
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
        let trigger = dummy_trigger();

        handle_input_action(InputAction::Select, &mut state, &trigger);
        assert!(state.tree.node(dir_id).unwrap().expanded);

        handle_input_action(InputAction::Select, &mut state, &trigger);
        assert!(!state.tree.node(dir_id).unwrap().expanded);
    }

    #[test]
    fn delete_action_first_press_requests_confirmation() {
        let mut state = sample_state();
        state.selection = Some(1);
        let trigger = dummy_trigger();
        handle_input_action(InputAction::Delete, &mut state, &trigger);
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
        adjust_ancestors_after_rescan(&mut state, Some(0), 10, 25);
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
}
