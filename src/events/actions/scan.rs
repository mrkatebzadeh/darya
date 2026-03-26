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

use crate::fs_scan::{ScanEvent, ScanNode};
use crate::size::total_size;
use crate::state::{AppState, StatusMessage, StatusOutcome};
use crate::tree::{FileTree, NodeMetadata, NodeType};
use std::path::PathBuf;

pub(crate) fn process_scan_event(state: &mut AppState, event: ScanEvent) {
    match event {
        ScanEvent::Batch(batch) => {
            let last_path = batch.nodes.last().map(|n| n.path.clone());
            state.scan_accumulator.push_batch(batch.nodes);
            if let Some(path) = last_path {
                state.update_status(StatusMessage::ScanPath(path));
            }
            if let Some(progress) = batch.progress {
                state.mark_scan_progress(progress.clone());
                state.update_status(StatusMessage::ScanProgress {
                    scanned: progress.scanned,
                    errors: progress.errors,
                });
            } else {
                state.mark_scan_complete();
            }
            if let Some(activity) = batch.activity {
                state.scan_activity = activity;
            }
        }
        ScanEvent::Node(node) => {
            state.scan_accumulator.push_node(node);
        }
        ScanEvent::Activity(activity) => {
            state.scan_activity = activity;
        }
        ScanEvent::Progress(progress) => {
            state.mark_scan_progress(progress.clone());
            state.update_status(StatusMessage::ScanProgress {
                scanned: progress.scanned,
                errors: progress.errors,
            });
        }
        ScanEvent::Error(error) => {
            state.mark_scan_error(format!("{}: {}", error.path.display(), error.source));
        }
        ScanEvent::Completed => {
            state.mark_scan_complete();
            state.update_status(StatusMessage::ScanComplete);
            rebuild_tree_from_pending(state);
        }
    }

    state.refresh_ui();
}

pub(crate) fn rescan_selection(state: &mut AppState) {
    if !state.allow_modifications {
        state.update_status(StatusMessage::ImportReadOnly);
        return;
    }
    let Some(selected_id) = state.selection else {
        return;
    };
    let Some(node) = state.tree.node(selected_id).cloned() else {
        return;
    };

    let previous_size = node.size;
    let node_path = node.path.clone();
    let refreshed_size = if node.file_type == NodeType::Directory {
        match total_size(&node.path, false) {
            Ok(size) => size,
            Err(err) => {
                state.update_status(StatusMessage::RescanResult {
                    path: node_path.clone(),
                    outcome: StatusOutcome::failure(err),
                });
                return;
            }
        }
    } else {
        match std::fs::metadata(&node.path) {
            Ok(meta) => meta.len(),
            Err(err) => {
                state.update_status(StatusMessage::RescanResult {
                    path: node_path.clone(),
                    outcome: StatusOutcome::failure(err),
                });
                return;
            }
        }
    };

    if let Some(current) = state.tree.node_mut(selected_id) {
        current.size = refreshed_size;
    }
    adjust_ancestors_after_rescan(state, node.parent, previous_size, refreshed_size);
    state.update_status(StatusMessage::RescanResult {
        path: node_path,
        outcome: StatusOutcome::Success,
    });
}

pub(crate) fn adjust_ancestors_after_rescan(
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

fn insert_scan_node(state: &mut AppState, node: &ScanNode) -> Option<(PathBuf, Option<usize>)> {
    let path = node.path.clone();
    let node_id = state.tree.ensure_node(path.clone(), node.kind);
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
    let parent = state.tree.node(node_id).and_then(|node| node.parent);
    Some((path, parent))
}

fn rebuild_tree_from_pending(state: &mut AppState) {
    let selected_path = state
        .selection
        .and_then(|id| state.tree.node(id).map(|node| node.path.clone()));
    let root_path = selected_path.clone().unwrap_or_else(|| {
        state
            .tree
            .node(state.tree.root())
            .map(|node| node.path.clone())
            .unwrap_or_else(|| PathBuf::from("/"))
    });
    state.tree = FileTree::new(root_path);
    let mut parents = Vec::new();
    let drained_nodes: Vec<ScanNode> = state.scan_accumulator.drain();
    for node in drained_nodes {
        if let Some((_path, Some(parent_id))) = insert_scan_node(state, &node) {
            parents.push(parent_id);
        }
    }
    parents.sort_unstable();
    parents.dedup();
    for parent in parents {
        state.tree.sort_children(parent, state.sort_mode);
    }
    state.tree.recompute_sizes();
    debug_assert!(
        state.tree.verify_size_invariants(),
        "tree size invariant violated after rebuilding"
    );
    restore_selection(state, selected_path);
}

fn restore_selection(state: &mut AppState, path: Option<PathBuf>) {
    if let Some(path) = path
        && let Some(id) = state.tree.node_id_for_path(&path)
    {
        state.selection = Some(id);
        return;
    }
    state.selection = Some(state.tree.root());
}
