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
    config::SortMode,
    display::DisplayOptions,
    fs_scan::{ScanActivity, ScanProgress},
    snapshot::ExportOptions,
    tree::{FileTree, NodeId, NodeType},
    treemap::TreemapNode,
};

use crate::fs_scan::ScanNode;
use std::path::PathBuf;

/// Tracks the current phase of the filesystem scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Running(ScanProgress),
    Completed,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeDisplayMode {
    Apparent,
    Disk,
}

/// Central application state shared across the UI and scanner.
#[derive(Debug)]
pub struct AppState {
    pub tree: FileTree,
    pub sort_mode: SortMode,
    pub selection: Option<NodeId>,
    pub scroll_offset: usize,
    pub scan_state: ScanState,
    pub scan_activity: ScanActivity,
    pub status_message: Option<String>,
    pub spinner_phase: usize,
    pub pending_delete: Option<NodeId>,
    pub size_mode: SizeDisplayMode,
    pub filter_query: String,
    pub filter_active: bool,
    pub filter_prompt_active: bool,
    pub show_help: bool,
    pub treemap_nodes: Vec<TreemapNode>,
    pub treemap_revision: u64,
    pub allow_modifications: bool,
    pub extended_mode: bool,
    pub display_options: DisplayOptions,
    pub export_options: ExportOptions,
    pub pending_scan_nodes: Vec<ScanNode>,
}

impl AppState {
    pub fn new(root: PathBuf, sort_mode: SortMode) -> Self {
        Self {
            tree: FileTree::new(root),
            sort_mode,
            selection: None,
            scroll_offset: 0,
            scan_state: ScanState::Idle,
            scan_activity: ScanActivity::default(),
            status_message: None,
            spinner_phase: 0,
            pending_delete: None,
            size_mode: SizeDisplayMode::Apparent,
            filter_query: String::new(),
            filter_active: false,
            show_help: false,
            filter_prompt_active: false,
            treemap_nodes: Vec::new(),
            treemap_revision: 0,
            allow_modifications: true,
            extended_mode: false,
            display_options: DisplayOptions::default(),
            export_options: ExportOptions::default(),
            pending_scan_nodes: Vec::new(),
        }
    }

    pub fn set_extended_mode(&mut self, enabled: bool) {
        self.extended_mode = enabled;
    }

    pub fn set_display_options(&mut self, options: DisplayOptions) {
        self.display_options = options;
    }

    pub fn set_export_options(&mut self, options: ExportOptions) {
        self.export_options = options;
    }

    pub fn select_node(&mut self, node: NodeId) {
        self.selection = Some(node);
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    pub fn update_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn set_sort_mode(&mut self, mode: SortMode) {
        self.sort_mode = mode;
        self.tree.sort_children(self.tree.root(), mode);
    }

    pub fn mark_scan_progress(&mut self, progress: ScanProgress) {
        self.scan_state = ScanState::Running(progress);
    }

    pub fn advance_spinner(&mut self, modulo: usize) {
        if modulo > 0 {
            self.spinner_phase = (self.spinner_phase + 1) % modulo;
        }
    }

    pub fn mark_scan_complete(&mut self) {
        self.scan_state = ScanState::Completed;
    }

    pub fn mark_scan_error(&mut self, message: impl Into<String>) {
        self.scan_state = ScanState::Error(message.into());
    }

    pub fn toggle_size_mode(&mut self) {
        self.size_mode = match self.size_mode {
            SizeDisplayMode::Apparent => SizeDisplayMode::Disk,
            SizeDisplayMode::Disk => SizeDisplayMode::Apparent,
        };
    }

    pub fn clear_filter(&mut self) {
        self.filter_prompt_active = false;
        self.filter_query.clear();
        self.filter_active = false;
    }

    pub fn scan_activity_snapshot(&self) -> ScanActivity {
        self.scan_activity.clone()
    }

    pub fn refresh_treemap_nodes(&mut self) {
        let source_id = self.tree.root();

        let Some(source) = self.tree.node(source_id) else {
            self.treemap_nodes.clear();
            self.treemap_revision = self.treemap_revision.wrapping_add(1);
            return;
        };

        let mut nodes: Vec<TreemapNode> = source
            .children
            .iter()
            .filter_map(|child_id| self.tree.node(*child_id))
            .filter_map(|child| {
                let size = match self.size_mode {
                    SizeDisplayMode::Apparent => child.size,
                    SizeDisplayMode::Disk => child.disk_size,
                };

                (size > 0).then(|| TreemapNode {
                    node_id: child.id,
                    name: child.name.clone(),
                    size,
                    is_directory: child.file_type == NodeType::Directory,
                    is_aggregated: false,
                })
            })
            .collect();

        nodes.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        self.treemap_nodes = nodes;
        self.treemap_revision = self.treemap_revision.wrapping_add(1);
        self.ensure_selection_visible();
    }

    pub fn ensure_selection_visible(&mut self) {
        let ids = self.visible_node_ids();
        if ids.is_empty() {
            self.selection = Some(self.tree.root());
            return;
        }
        if self
            .selection
            .is_some_and(|selection| ids.contains(&selection))
        {
            return;
        }
        self.selection = Some(ids[0]);
    }

    pub fn visible_node_ids(&self) -> Vec<NodeId> {
        self.tree
            .visible_ids()
            .into_iter()
            .filter(|id| {
                if self.display_options.show_hidden {
                    return true;
                }
                if let Some(node) = self.tree.node(*id) {
                    !node.name.starts_with('.')
                } else {
                    true
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn state() -> AppState {
        AppState::new(PathBuf::from("/"), SortMode::SizeDesc)
    }

    #[test]
    fn new_state_is_idle() {
        let state = state();
        assert_eq!(state.scan_state, ScanState::Idle);
        assert!(state.selection.is_none());
        assert_eq!(state.sort_mode, SortMode::SizeDesc);
        assert!(state.treemap_nodes.is_empty());
        assert!(state.allow_modifications);
        assert!(!state.extended_mode);
        assert_eq!(state.export_options, ExportOptions::default());
        assert_eq!(state.display_options, DisplayOptions::default());
    }

    #[test]
    fn selection_and_scroll_updated() {
        let mut state = state();
        state.select_node(2);
        assert_eq!(state.selection, Some(2));
        state.set_scroll_offset(10);
        assert_eq!(state.scroll_offset, 10);
        state.clear_selection();
        assert!(state.selection.is_none());
    }

    #[test]
    fn scan_state_transitions() {
        let mut state = state();
        state.mark_scan_progress(ScanProgress {
            scanned: 1,
            errors: 0,
        });
        assert!(matches!(state.scan_state, ScanState::Running(_)));
        state.mark_scan_complete();
        assert_eq!(state.scan_state, ScanState::Completed);
        state.mark_scan_error("oops");
        assert!(matches!(state.scan_state, ScanState::Error(_)));
    }

    #[test]
    fn status_message_behaves() {
        let mut state = state();
        state.update_status("hello");
        assert_eq!(state.status_message.as_deref(), Some("hello"));
        state.clear_status();
        assert!(state.status_message.is_none());
    }
}
