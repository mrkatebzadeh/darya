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

use crate::snapshot::{self, SnapshotEndpoint, SnapshotFormat};
use crate::state::{AppState, StatusMessage, StatusOutcome};
use std::fs;
use std::process::Command;

pub(crate) fn delete_selection(state: &mut AppState) {
    if !state.allow_modifications {
        state.update_status(StatusMessage::ImportReadOnly);
        return;
    }
    let Some(selected_id) = state.navigation.selection else {
        return;
    };

    if state.pending_delete != Some(selected_id) {
        state.pending_delete = Some(selected_id);
        if let Some(node) = state.tree.node(selected_id) {
            state.update_status(StatusMessage::DeleteConfirmation(node.path.clone()));
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
            state.update_status(StatusMessage::DeleteSuccess(target.clone()));
        }
        Err(err) => {
            state.mark_scan_error(format!("delete failed for {}: {err}", target.display()));
            state.update_status(StatusMessage::DeleteFailure(target.clone()));
        }
    }
    state.pending_delete = None;
}

pub(crate) fn open_selection(state: &mut AppState) {
    if !state.allow_modifications {
        state.update_status(StatusMessage::ImportReadOnly);
        return;
    }
    let Some(selected_id) = state.navigation.selection else {
        return;
    };
    let Some(path) = state.tree.node(selected_id).map(|n| n.path.clone()) else {
        return;
    };

    let mut command = open_command_for_platform();
    command.arg(&path);

    match command.spawn() {
        Ok(_) => state.update_status(StatusMessage::OpenResult {
            path: path.clone(),
            outcome: StatusOutcome::Success,
        }),
        Err(err) => state.update_status(StatusMessage::OpenResult {
            path: path.clone(),
            outcome: StatusOutcome::failure(err),
        }),
    }
}

pub(crate) fn export_scan(state: &mut AppState) {
    let snapshot_path = std::path::Path::new("/tmp/darya-scan.json");
    match snapshot::export_tree(&state.tree, snapshot_path, state.export_options) {
        Ok(()) => state.update_status(StatusMessage::ExportResult {
            path: snapshot_path.to_path_buf(),
            outcome: StatusOutcome::Success,
        }),
        Err(err) => state.update_status(StatusMessage::ExportResult {
            path: snapshot_path.to_path_buf(),
            outcome: StatusOutcome::failure(err),
        }),
    }
}

pub(crate) fn import_scan(state: &mut AppState) {
    let snapshot_path = std::path::Path::new("/tmp/darya-scan.json");
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
            state.navigation.selection = Some(state.tree.root());
            state.update_status(StatusMessage::ImportResult {
                path: snapshot_path.to_path_buf(),
                outcome: StatusOutcome::Success,
            });
        }
        Err(err) => state.update_status(StatusMessage::ImportResult {
            path: snapshot_path.to_path_buf(),
            outcome: StatusOutcome::failure(err),
        }),
    }
}

pub(crate) fn open_command_for_platform() -> Command {
    if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    }
}
