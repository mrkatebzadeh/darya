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
use crate::display::DisplayOptions;
use crate::state::{AppState, SizeDisplayMode};
use crate::theme::Theme;
use crate::tree::{FileTree, NodeType, TreeNode};
use crate::treemap::TreemapNode;
use ratatui::layout::Rect;
#[cfg(test)]
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Row};
#[cfg(test)]
use std::time::UNIX_EPOCH;
use throbber_widgets_tui::BRAILLE_EIGHT;

pub(crate) fn collect_tree_rows(
    tree: &FileTree,
    filter_query: &str,
    filter_active: bool,
    options: DisplayOptions,
) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    traverse(tree, tree.root(), 0, &mut rows, options);
    if filter_active && !filter_query.is_empty() {
        let filter = filter_query.to_lowercase();
        rows.retain(|row| row.name.to_lowercase().contains(&filter));
    }
    rows
}

#[derive(Clone, Copy)]
pub(crate) struct ColumnWidths {
    pub percent: usize,
    pub size: usize,
}

fn traverse(
    tree: &FileTree,
    id: usize,
    depth: usize,
    rows: &mut Vec<TreeRow>,
    options: DisplayOptions,
) {
    if let Some(node) = tree.node(id) {
        if depth > 0 && !options.show_hidden && node.name.starts_with('.') {
            return;
        }

        rows.push(TreeRow {
            id: node.id,
            depth,
            name: node.name.clone(),
            size: node.size,
            disk_size: node.disk_size,
            kind: node.file_type,
        });

        if node.expanded {
            for &child in &node.children {
                traverse(tree, child, depth + 1, rows, options);
            }
        }
    }
}

pub(crate) fn build_row(
    row: &TreeRow,
    selection: Option<usize>,
    theme: Theme,
    size_mode: SizeDisplayMode,
    max_size: u64,
    _options: DisplayOptions,
    column_widths: ColumnWidths,
) -> Row<'static> {
    let indent = "  ".repeat(row.depth);
    let icon = match row.kind {
        NodeType::Directory => "📁",
        NodeType::File => "📄",
        NodeType::Symlink => "🔗",
        NodeType::Other => "❓",
    };

    let style = if Some(row.id) == selection {
        ratatui::style::Style::default()
            .bg(theme.selection)
            .fg(theme.background)
    } else {
        ratatui::style::Style::default()
            .bg(theme.background)
            .fg(theme.foreground)
    };

    let size_value = chosen_size(row, size_mode, _options);
    let size_label = format_size_custom(size_value, _options.use_si);
    let percent = if max_size == 0 {
        0.0
    } else {
        size_value as f64 / max_size as f64 * 100.0
    };
    let percent_column_width = column_widths.percent;
    let size_column_width = column_widths.size;
    let percent_label = format!("{percent:>6.1}%");
    let trimmed_percent_label = trim_to_width(&percent_label, percent_column_width);
    let bar_width = percent_column_width.saturating_sub(trimmed_percent_label.len());
    let bar = percent_bar(percent, bar_width);
    let percent_bar_style = if Some(row.id) == selection {
        // Let the row highlight background show through.
        Style::default().fg(theme.bar)
    } else {
        Style::default().fg(theme.bar).bg(theme.bar_bg)
    };
    let percent_value_style = Style::default().fg(theme.foreground);
    let size_value_style = Style::default().fg(theme.bar);
    let percent_cell = if percent_column_width == 0 {
        Cell::from(Span::raw(String::new()))
    } else {
        let combined_len = bar_width + trimmed_percent_label.len();
        let padding = percent_column_width.saturating_sub(combined_len);
        let mut spans = Vec::new();
        if padding > 0 {
            spans.push(Span::raw(" ".repeat(padding)));
        }
        if !bar.is_empty() {
            spans.push(Span::styled(bar.clone(), percent_bar_style));
        }
        if !trimmed_percent_label.is_empty() {
            spans.push(Span::styled(
                trimmed_percent_label.clone(),
                percent_value_style,
            ));
        }
        Cell::from(Line::from(spans))
    };
    let trimmed_size_label = trim_to_width(&size_label, size_column_width);
    let size_column_content = if size_column_width == 0 {
        String::new()
    } else {
        format!("{trimmed_size_label:>width$}", width = size_column_width)
    };

    let mut cells = vec![Cell::from(format!("{}{} {}", indent, icon, row.name))];
    let size_cell = Cell::from(Span::styled(size_column_content, size_value_style));
    cells.push(percent_cell);
    cells.push(size_cell);

    Row::new(cells).style(style)
}

pub(crate) fn chosen_size(row: &TreeRow, mode: SizeDisplayMode, options: DisplayOptions) -> u64 {
    if options.prefer_disk {
        row.disk_size
    } else {
        match mode {
            SizeDisplayMode::Apparent => row.size,
            SizeDisplayMode::Disk => row.disk_size,
        }
    }
}

fn percent_bar(percent: f64, width: usize) -> String {
    let ratio = (percent.clamp(0.0, 100.0) / 100.0).min(1.0);
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "-".repeat(empty))
}

pub(crate) fn trim_to_width(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if value.len() <= width {
        value.to_string()
    } else {
        value.chars().take(width).collect()
    }
}

fn format_size_custom(bytes: u64, use_si: bool) -> String {
    let (unit, div) = if use_si {
        ("kB", 1000.0)
    } else {
        ("KiB", 1024.0)
    };
    let value = bytes as f64;
    if value >= div * div * div {
        format!(
            "{:.1} {}",
            value / (div * div * div),
            unit.replace('k', "G")
        )
    } else if value >= div * div {
        format!("{:.1} {}", value / (div * div), unit.replace('k', "M"))
    } else if value >= div {
        format!("{:.1} {}", value / div, unit)
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn spinner_symbol(phase: usize) -> &'static str {
    let symbols = BRAILLE_EIGHT.symbols;
    symbols[phase % symbols.len()]
}

#[cfg(test)]
pub(crate) fn selected_info_line(state: &AppState) -> String {
    let Some(selected) = state.selection else {
        return "info: no selection".to_string();
    };

    let Some(node) = state.tree.node(selected) else {
        return "info: no node".to_string();
    };

    if state.extended_mode
        && let Some(line) = format_node_metadata(node)
    {
        return line;
    }

    let Ok(metadata) = std::fs::metadata(&node.path) else {
        return format!("info: metadata unavailable for {}", node.path.display());
    };

    let modified = metadata
        .modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|| "-".to_string());

    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let perm = metadata.permissions().mode() & 0o777;
        format!(
            "info: mode={perm:o} uid={} gid={} inode={} nlink={} mtime={} ctime={}",
            metadata.uid(),
            metadata.gid(),
            metadata.ino(),
            metadata.nlink(),
            modified,
            metadata.ctime()
        )
    }

    #[cfg(not(unix))]
    {
        format!(
            "info: readonly={} mtime={modified}",
            metadata.permissions().readonly()
        )
    }
}

#[cfg(test)]
fn format_node_metadata(node: &TreeNode) -> Option<String> {
    let modified = node
        .modified
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|| "-".to_string());

    #[cfg(unix)]
    {
        if let (Some(mode), Some(uid), Some(gid)) = (node.permissions, node.uid, node.gid) {
            return Some(format!(
                "info: mode={mode:o} uid={uid} gid={gid} mtime={modified}",
            ));
        }
    }

    #[cfg(not(unix))]
    {
        if let Some(mode) = node.permissions {
            return Some(format!("info: mode={mode:o} mtime={modified}"));
        }
    }

    None
}

pub(crate) fn detail_panel_lines(state: &AppState) -> Vec<String> {
    let empty = || {
        vec![
            detail_line("•", "path", ""),
            detail_line("•", "type", ""),
            detail_line("•", "apparent", ""),
            detail_line("•", "disk", ""),
            detail_line("•", "items", ""),
        ]
    };

    let Some(selected) = state.selection else {
        return empty();
    };

    let Some(node) = state.tree.node(selected) else {
        return empty();
    };

    let apparent = format_size_custom(node.size, state.display_options.use_si);
    let disk = format_size_custom(node.disk_size, state.display_options.use_si);
    let ratio = if node.size == 0 {
        0.0
    } else {
        (node.disk_size as f64 / node.size as f64) * 100.0
    };
    let kind_label = match node.file_type {
        NodeType::Directory => "directory",
        NodeType::File => "file",
        NodeType::Symlink => "symlink",
        NodeType::Other => "other",
    };

    vec![
        detail_line("📂", "path", &node.path.display().to_string()),
        detail_line("⬤", "type", kind_label),
        detail_line("⚖", "apparent", &apparent),
        detail_line("💾", "disk", &format!("{disk} ({ratio:.1}%)")),
        if node.file_type == NodeType::Directory {
            detail_line("📦", "items", &node.children.len().to_string())
        } else {
            detail_line("📦", "items", "")
        },
    ]
}

fn detail_line(glyph: &str, key: &str, value: &str) -> String {
    format!("{glyph}\t{key}:\t{value}")
}

pub(crate) fn sort_mode_label(mode: SortMode) -> &'static str {
    match mode {
        SortMode::SizeDesc => "size_desc",
        SortMode::SizeAsc => "size_asc",
        SortMode::Name => "name",
        SortMode::ModifiedTime => "modified_time",
    }
}

pub(crate) fn fill_rect(frame: &mut Frame<'_>, rect: Rect, fg: Color, bg: Color) {
    let buf = frame.buffer_mut();
    let x_end = rect.x.saturating_add(rect.width);
    let y_end = rect.y.saturating_add(rect.height);

    for y in rect.y..y_end {
        for x in rect.x..x_end {
            let cell = buf.get_mut(x, y);
            cell.set_symbol("█");
            cell.set_fg(fg);
            cell.set_bg(bg);
        }
    }
}

pub(crate) fn selection_path(state: &AppState) -> Vec<usize> {
    let Some(mut current) = state.selection else {
        return Vec::new();
    };
    let root = state.tree.root();
    let mut stack = Vec::new();

    while current != root {
        stack.push(current);
        let parent = state.tree.node(current).and_then(|node| node.parent);
        if let Some(parent) = parent {
            current = parent;
        } else {
            break;
        }
    }

    stack.reverse();
    stack
}

pub(crate) fn gather_child_nodes(
    parent_id: usize,
    state: &AppState,
    max_children: usize,
) -> Vec<TreemapNode> {
    let mut nodes = Vec::new();
    let parent = match state.tree.node(parent_id) {
        Some(node) => node,
        None => return nodes,
    };

    for &child_id in &parent.children {
        if let Some(child) = state.tree.node(child_id) {
            let size = node_size(child, state.size_mode);
            if size == 0 {
                continue;
            }

            nodes.push(TreemapNode {
                node_id: child.id,
                name: child.name.clone(),
                size,
                is_directory: child.file_type == NodeType::Directory,
                is_aggregated: false,
            });
        }
    }

    nodes.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    let limit = max_children.max(1);
    nodes.truncate(limit);
    nodes
}

fn node_size(node: &TreeNode, mode: SizeDisplayMode) -> u64 {
    match mode {
        SizeDisplayMode::Apparent => node.size,
        SizeDisplayMode::Disk => node.disk_size,
    }
}

#[cfg(test)]
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub(crate) struct TreeRow {
    id: usize,
    depth: usize,
    name: String,
    size: u64,
    disk_size: u64,
    kind: NodeType,
}

impl TreeRow {
    pub fn id(&self) -> usize {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{NodeType, TreeNode};
    use std::{fs, path::PathBuf};

    #[test]
    fn flatten_tree_respects_depth() {
        let mut tree = FileTree::new(PathBuf::from("/root"));
        let child = tree.add_child(
            0,
            TreeNode::new(PathBuf::from("/root/file"), NodeType::File),
        );
        let rows = flatten_rows(&tree);
        assert!(rows.iter().any(|row| row.id == child));
        let depth = rows.into_iter().find(|row| row.id == child).unwrap().depth;
        assert_eq!(depth, 1);
    }

    #[test]
    fn collapsed_nodes_hidden() {
        let mut tree = FileTree::new(PathBuf::from("/root"));
        let child = tree.add_child(
            0,
            TreeNode::new(PathBuf::from("/root/file"), NodeType::File),
        );
        if let Some(root) = tree.node_mut(0) {
            root.expanded = false;
        }
        let ids = flatten_ids(&tree);
        assert_eq!(ids, vec![0]);
        assert!(!ids.contains(&child));
    }

    fn flatten_rows(tree: &FileTree) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        traverse(tree, tree.root(), 0, &mut rows, DisplayOptions::default());
        rows
    }

    fn flatten_ids(tree: &FileTree) -> Vec<usize> {
        flatten_rows(tree).into_iter().map(|row| row.id).collect()
    }

    #[test]
    fn selected_info_contains_metadata_fields() {
        let temp = std::env::temp_dir().join("dar-info-panel-test");
        fs::write(&temp, b"x").unwrap();

        let mut state =
            crate::state::AppState::new(PathBuf::from("/"), crate::config::SortMode::SizeDesc);
        let file_id = state
            .tree
            .add_child(0, TreeNode::new(temp.clone(), NodeType::File));
        state.selection = Some(file_id);

        let info = selected_info_line(&state);
        assert!(info.contains("mtime="));
        #[cfg(unix)]
        assert!(info.contains("uid="));

        let _ = fs::remove_file(temp);
    }

    #[test]
    fn centered_rect_is_inside_parent() {
        let outer = Rect::new(0, 0, 100, 40);
        let inner = centered_rect(80, 70, outer);
        assert!(inner.width < outer.width);
        assert!(inner.height < outer.height);
        assert!(inner.x > outer.x);
        assert!(inner.y > outer.y);
    }
}
