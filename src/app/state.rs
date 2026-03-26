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

use super::scan_accumulator::ScanAccumulator;
use crate::{
    config::SortMode,
    display::DisplayOptions,
    fs_scan::{ScanActivity, ScanProgress},
    snapshot::ExportOptions,
    tree::{FileTree, NodeId, NodeType},
    treemap::TreemapNode,
};
use std::fmt;
use std::path::PathBuf;

/// Tracks the current phase of the filesystem scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanState {
    Idle,
    Running(ScanProgress),
    Completed,
    Error(String),
}

/// High-level outcome of an action that can succeed or fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusOutcome {
    Success,
    Failure(String),
}

impl StatusOutcome {
    pub fn failure(message: impl fmt::Display) -> Self {
        Self::Failure(message.to_string())
    }
}

/// Structured status messages that drive the footer line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusMessage {
    FilterPrompt,
    FilterActive(String),
    FilterCleared,
    SortMode(SortMode),
    HiddenFilesVisible(bool),
    HelpOpened,
    HelpClosed,
    ScanHint(PathBuf),
    ScanPath(PathBuf),
    ScanProgress {
        scanned: u64,
        errors: u64,
    },
    ScanComplete,
    ImportReadOnly,
    DeleteConfirmation(PathBuf),
    DeleteSuccess(PathBuf),
    DeleteFailure(PathBuf),
    OpenResult {
        path: PathBuf,
        outcome: StatusOutcome,
    },
    ExportResult {
        path: PathBuf,
        outcome: StatusOutcome,
    },
    ImportResult {
        path: PathBuf,
        outcome: StatusOutcome,
    },
    RescanResult {
        path: PathBuf,
        outcome: StatusOutcome,
    },
    Custom(String),
}

impl fmt::Display for StatusMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatusMessage::FilterPrompt => write!(f, "filter: type name substring and press Enter"),
            StatusMessage::FilterActive(query) => write!(f, "filter active: {query}"),
            StatusMessage::FilterCleared => write!(f, "filter cleared"),
            StatusMessage::SortMode(mode) => write!(f, "sort mode: {}", mode.as_label()),
            StatusMessage::HiddenFilesVisible(true) => write!(f, "hidden files shown"),
            StatusMessage::HiddenFilesVisible(false) => write!(f, "hidden files hidden"),
            StatusMessage::HelpOpened => write!(f, "help opened"),
            StatusMessage::HelpClosed => write!(f, "help closed"),
            StatusMessage::ScanHint(path) => write!(f, "press R to scan {}", path.display()),
            StatusMessage::ScanPath(path) => write!(f, "scanned {}", path.display()),
            StatusMessage::ScanProgress { scanned, errors } => {
                write!(f, "scanned {scanned} entries, {errors} errors")
            }
            StatusMessage::ScanComplete => write!(f, "scan complete"),
            StatusMessage::ImportReadOnly => write!(f, "imported scan is read-only"),
            StatusMessage::DeleteConfirmation(path) => {
                write!(f, "press d again to delete {}", path.display())
            }
            StatusMessage::DeleteSuccess(path) => write!(f, "deleted {}", path.display()),
            StatusMessage::DeleteFailure(path) => write!(f, "delete failed: {}", path.display()),
            StatusMessage::OpenResult { path, outcome } => match outcome {
                StatusOutcome::Success => write!(f, "opened {}", path.display()),
                StatusOutcome::Failure(err) => {
                    write!(f, "open failed for {}: {err}", path.display())
                }
            },
            StatusMessage::ExportResult { path, outcome } => match outcome {
                StatusOutcome::Success => write!(f, "scan exported to {}", path.display()),
                StatusOutcome::Failure(err) => write!(f, "export failed: {err}"),
            },
            StatusMessage::ImportResult { path, outcome } => match outcome {
                StatusOutcome::Success => write!(f, "scan imported from {}", path.display()),
                StatusOutcome::Failure(err) => write!(f, "import failed: {err}"),
            },
            StatusMessage::RescanResult { path, outcome } => match outcome {
                StatusOutcome::Success => write!(f, "rescanned {}", path.display()),
                StatusOutcome::Failure(err) => write!(f, "rescan failed: {err}"),
            },
            StatusMessage::Custom(text) => write!(f, "{text}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeDisplayMode {
    Apparent,
    Disk,
}

#[derive(Debug, Clone, Default)]
pub struct FilterState {
    pub query: String,
    pub active: bool,
    pub prompt_active: bool,
}

impl FilterState {
    pub fn clear(&mut self) {
        self.query.clear();
        self.active = false;
        self.prompt_active = false;
    }
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
    pub status_message: Option<StatusMessage>,
    pub spinner_phase: usize,
    pub pending_delete: Option<NodeId>,
    pub size_mode: SizeDisplayMode,
    pub filter: FilterState,
    pub show_help: bool,
    pub treemap_visible: bool,
    pub treemap_nodes: Vec<TreemapNode>,
    pub treemap_revision: u64,
    pub ui_revision: u64,
    pub allow_modifications: bool,
    pub extended_mode: bool,
    pub display_options: DisplayOptions,
    pub export_options: ExportOptions,
    pub scan_accumulator: ScanAccumulator,
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
            filter: FilterState::default(),
            show_help: false,
            treemap_visible: true,
            treemap_nodes: Vec::new(),
            treemap_revision: 0,
            ui_revision: 0,
            allow_modifications: true,
            extended_mode: false,
            display_options: DisplayOptions::default(),
            export_options: ExportOptions::default(),
            scan_accumulator: ScanAccumulator::default(),
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

    pub fn update_status(&mut self, message: StatusMessage) {
        self.status_message = Some(message);
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn status_text(&self) -> String {
        self.status_message
            .as_ref()
            .map(|message| message.to_string())
            .unwrap_or_else(|| "ready".to_string())
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

    pub fn toggle_treemap_visibility(&mut self) {
        self.treemap_visible = !self.treemap_visible;
        let message = if self.treemap_visible {
            "treemap panel shown"
        } else {
            "treemap panel hidden"
        };
        self.update_status(StatusMessage::Custom(message.to_string()));
    }

    pub fn is_treemap_visible(&self) -> bool {
        self.treemap_visible
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
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

    pub fn mark_ui_dirty(&mut self) {
        self.ui_revision = self.ui_revision.wrapping_add(1);
    }

    pub fn refresh_ui(&mut self) {
        self.refresh_treemap_nodes();
        self.mark_ui_dirty();
    }

    pub fn ui_revision(&self) -> u64 {
        self.ui_revision
    }

    pub fn visible_node_ids(&self) -> Vec<NodeId> {
        self.tree
            .visible_ids_filtered(self.display_options.show_hidden)
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
        state.update_status(StatusMessage::Custom("hello".to_string()));
        assert_eq!(
            state.status_message,
            Some(StatusMessage::Custom("hello".to_string()))
        );
        assert_eq!(state.status_text(), "hello");
        state.clear_status();
        assert!(state.status_message.is_none());
    }
}
