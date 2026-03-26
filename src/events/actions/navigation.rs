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

use crate::state::AppState;
use crate::tree::NodeType;

pub(crate) fn select_previous(state: &mut AppState) {
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

pub(crate) fn select_next(state: &mut AppState) {
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

pub(crate) fn select_first(state: &mut AppState) {
    let ids = state.visible_node_ids();
    if let Some(&first) = ids.first() {
        state.selection = Some(first);
    }
}

pub(crate) fn select_last(state: &mut AppState) {
    let ids = state.visible_node_ids();
    if let Some(&last) = ids.last() {
        state.selection = Some(last);
    }
}

pub(crate) fn expand_selection(state: &mut AppState) {
    if let Some(id) = state.selection
        && let Some(node) = state.tree.node_mut(id)
        && node.file_type == NodeType::Directory
    {
        node.expanded = true;
    }
}

pub(crate) fn collapse_selection(state: &mut AppState) {
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

pub(crate) fn toggle_selection(state: &mut AppState) {
    if let Some(id) = state.selection
        && let Some(node) = state.tree.node_mut(id)
        && node.file_type == NodeType::Directory
    {
        node.expanded = !node.expanded;
    }
}
