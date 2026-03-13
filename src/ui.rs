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
    layout::LayoutRegions,
    state::{AppState, ScanState, SizeDisplayMode},
    theme::Theme,
    tree::{FileTree, NodeType, TreeNode},
    treemap::{TreemapNode, TreemapTile, squarified_treemap},
};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    terminal::Frame,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};
use std::time::UNIX_EPOCH;
use throbber_widgets_tui::BRAILLE_EIGHT;

/// Renderer responsible for drawing the main UI panels.
pub struct Ui;

impl Ui {
    pub fn draw(
        &self,
        frame: &mut Frame<'_>,
        layout: LayoutRegions,
        state: &AppState,
        theme: Theme,
    ) {
        self.draw_header(frame, layout.header, state, theme);
        self.draw_tree(frame, layout.tree, state, theme);
        self.draw_treemap(frame, layout.treemap, state, theme);
        self.draw_details(frame, layout.details, theme);
        self.draw_footer(frame, layout.footer, state, theme);
        if state.show_help {
            self.draw_help_modal(frame, state, theme);
        }
    }

    fn draw_header(&self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        let root_label = state
            .tree
            .node(state.tree.root())
            .map(|node| node.path.display().to_string())
            .unwrap_or_else(|| "<unknown>".into());

        let progress_label = match &state.scan_state {
            ScanState::Running(progress) => {
                let spinner = spinner_symbol(state.spinner_phase);
                format!(
                    "{spinner} please wait — scanned {} err {}",
                    progress.scanned, progress.errors
                )
            }
            ScanState::Error(message) => format!("error: {message}"),
            ScanState::Completed => "scan complete".into(),
            _ => "scan idle".into(),
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled("root: ", Style::default().fg(theme.directory)),
            Span::styled(root_label, Style::default().fg(theme.foreground)),
            Span::raw(format!(" | sort:{} ", sort_mode_label(state.sort_mode))),
            Span::styled(progress_label, Style::default().fg(theme.selection)),
        ]))
        .block(Block::default().borders(Borders::ALL).title("dar"))
        .style(Style::default().bg(theme.background));

        frame.render_widget(header, area);
    }

    fn draw_tree(&self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        let rows = build_rows(
            &state.tree,
            state.selection,
            theme,
            state.size_mode,
            &state.filter_query,
            state.filter_active,
        );
        let table = Table::new(rows)
            .block(Block::default().borders(Borders::ALL).title("filesystem"))
            .widths(&[
                Constraint::Percentage(55),
                Constraint::Length(12),
                Constraint::Percentage(33),
            ])
            .column_spacing(1);

        frame.render_widget(table, area);
    }

    fn draw_footer(&self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        let status_text = state.status_message.as_deref().unwrap_or("ready");
        let progress_label = match &state.scan_state {
            ScanState::Running(progress) => {
                let spinner = spinner_symbol(state.spinner_phase);
                format!(
                    "{spinner} scanning... {} entries, {} errors",
                    progress.scanned, progress.errors
                )
            }
            ScanState::Error(message) => format!("error: {message}"),
            ScanState::Completed => "scan complete".into(),
            _ => "scan idle".into(),
        };

        let footer = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("status: ", Style::default().fg(theme.foreground)),
                Span::raw(status_text.to_string()),
                Span::raw(" | "),
                Span::styled(progress_label, Style::default().fg(theme.selection)),
            ]),
            Line::from(Span::raw(selected_info_line(state))),
            Line::from(Span::raw(
                "hjkl: move │ gg/G: jump │ enter/tab: toggle │ d: delete │ o: open │ /: filter │ c: clear filter │ r: rescan │ b: size mode │ s: cycle sort │ E/I: export/import │ ?: help │ q: quit",
            )),
        ])
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().bg(theme.background));

        frame.render_widget(footer, area);
    }

    fn draw_treemap(&self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        let title = format!("treemap ({})", treemap_scope_name(state));
        let panel = Block::default().borders(Borders::ALL).title(title);
        let inner = panel.inner(area);
        frame.render_widget(panel, area);

        if inner.width < 2 || inner.height < 2 {
            return;
        }

        let max_tiles_for_panel = usize::from(inner.width) * usize::from(inner.height) / 2;
        let max_tiles = 200_usize.min(max_tiles_for_panel.max(1));
        let tiles = squarified_treemap(&state.treemap_nodes, inner, max_tiles);
        if tiles.is_empty() {
            frame.render_widget(
                Paragraph::new("No sized children")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.foreground).bg(theme.background)),
                inner,
            );
            return;
        }

        let path = selection_path(state);
        let highlighted_id = path.first().copied();
        let overlays = selection_highlight_overlays(&tiles, &path, state);

        for tile in tiles {
            let is_highlighted = Some(tile.node.node_id) == highlighted_id;
            self.draw_treemap_tile(frame, tile, theme, is_highlighted);
        }

        for overlay in overlays {
            fill_rect(frame, overlay, theme.selection, theme.background);
        }
    }

    fn draw_treemap_tile(
        &self,
        frame: &mut Frame<'_>,
        tile: TreemapTile,
        theme: Theme,
        is_highlighted: bool,
    ) {
        if tile.rect.width == 0 || tile.rect.height == 0 {
            return;
        }

        let color = if is_highlighted {
            theme.selection
        } else {
            theme.bar
        };
        fill_rect(frame, tile.rect, color, theme.background);
    }

    fn draw_details(&self, frame: &mut Frame<'_>, area: Rect, theme: Theme) {
        let panel = Paragraph::new("Details")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Details"))
            .style(Style::default().fg(theme.foreground).bg(theme.background));

        frame.render_widget(panel, area);
    }

    fn draw_help_modal(&self, frame: &mut Frame<'_>, _state: &AppState, theme: Theme) {
        let area = centered_rect(80, 70, frame.size());
        let lines = vec![
            Line::from("██████╗  █████╗ ██████╗"),
            Line::from("██╔══██╗██╔══██╗██╔══██╗"),
            Line::from("██║  ██║███████║██████╔╝"),
            Line::from("██║  ██║██╔══██║██╔══██╗"),
            Line::from("██████╔╝██║  ██║██║  ██║"),
            Line::from("╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝"),
            Line::from(""),
            Line::from("Keybindings:"),
            Line::from("  j/k or up/down: move"),
            Line::from("  enter/tab: toggle folder"),
            Line::from("  d: delete (with confirmation)"),
            Line::from("  o: open selected path"),
            Line::from("  /: start filter, c: clear filter"),
            Line::from("  b: size mode, s: sort mode, r: rescan"),
            Line::from("  E/I: export/import snapshot"),
            Line::from("  ?: toggle this help, q: quit"),
        ];

        let popup = Paragraph::new(lines)
            .block(Block::default().title("DAR Help").borders(Borders::ALL))
            .style(Style::default().fg(theme.foreground).bg(theme.background));

        frame.render_widget(Clear, area);
        frame.render_widget(popup, area);
    }
}

struct TreeRow {
    id: usize,
    depth: usize,
    name: String,
    size: u64,
    disk_size: u64,
    kind: NodeType,
}

fn build_rows(
    tree: &FileTree,
    selection: Option<usize>,
    theme: Theme,
    size_mode: SizeDisplayMode,
    filter_query: &str,
    filter_active: bool,
) -> Vec<Row<'static>> {
    let mut rows = Vec::new();
    traverse(tree, tree.root(), 0, &mut rows);
    if filter_active && !filter_query.is_empty() {
        let filter = filter_query.to_lowercase();
        rows.retain(|row| row.name.to_lowercase().contains(&filter));
    }
    let max_size = rows
        .iter()
        .map(|row| chosen_size(row, size_mode))
        .max()
        .unwrap_or(1);

    rows.into_iter()
        .map(|row| {
            let indent = "  ".repeat(row.depth);
            let icon = match row.kind {
                NodeType::Directory => "📁",
                NodeType::File => "📄",
                NodeType::Symlink => "🔗",
                NodeType::Other => "❓",
            };

            let style = if Some(row.id) == selection {
                Style::default().bg(theme.selection).fg(theme.background)
            } else {
                Style::default().bg(theme.background).fg(theme.foreground)
            };

            Row::new(vec![
                Cell::from(format!("{}{} {}", indent, icon, row.name)),
                Cell::from(format!(
                    "{} | d:{}",
                    format_size(row.size),
                    format_size(row.disk_size)
                )),
                Cell::from(draw_bar(chosen_size(&row, size_mode), max_size, 12)),
            ])
            .style(style)
        })
        .collect()
}

fn traverse(tree: &FileTree, id: usize, depth: usize, rows: &mut Vec<TreeRow>) {
    if let Some(node) = tree.node(id) {
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
                traverse(tree, child, depth + 1, rows);
            }
        }
    }
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = bytes as f64;

    if value >= GB {
        format!("{:.1} GiB", value / GB)
    } else if value >= MB {
        format!("{:.1} MiB", value / MB)
    } else if value >= KB {
        format!("{:.1} KiB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

fn draw_bar(size: u64, max: u64, width: usize) -> String {
    let filled = if max == 0 {
        0
    } else {
        let ratio = size as f64 / max as f64;
        ((ratio * width as f64).round() as usize).min(width)
    };

    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "#".repeat(filled), " ".repeat(empty))
}

fn spinner_symbol(phase: usize) -> &'static str {
    let symbols = BRAILLE_EIGHT.symbols;
    symbols[phase % symbols.len()]
}

fn chosen_size(row: &TreeRow, mode: SizeDisplayMode) -> u64 {
    match mode {
        SizeDisplayMode::Apparent => row.size,
        SizeDisplayMode::Disk => row.disk_size,
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn selected_info_line(state: &AppState) -> String {
    let Some(selected) = state.selection else {
        return "info: no selection".to_string();
    };

    let Some(node) = state.tree.node(selected) else {
        return "info: no node".to_string();
    };

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

fn treemap_scope_name(state: &AppState) -> String {
    let Some(root) = state.tree.node(state.tree.root()) else {
        return "root".to_string();
    };

    root.name.clone()
}

fn fill_rect(frame: &mut Frame<'_>, rect: Rect, fg: Color, bg: Color) {
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

fn selection_path(state: &AppState) -> Vec<usize> {
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

fn selection_highlight_overlays(
    tiles: &[TreemapTile],
    path: &[usize],
    state: &AppState,
) -> Vec<Rect> {
    if path.len() <= 1 {
        return Vec::new();
    }

    let root_tile = tiles.iter().find(|tile| tile.node.node_id == path[0]);
    let mut current_rect = match root_tile {
        Some(tile) => tile.rect,
        None => return Vec::new(),
    };

    let mut overlays = Vec::new();
    let mut parent_id = path[0];

    for &child_id in &path[1..] {
        let parent_node = state.tree.node(parent_id);
        let child_node = state.tree.node(child_id);

        if let (Some(parent_node), Some(child_node)) = (parent_node, child_node) {
            let parent_size = node_size(parent_node, state.size_mode);
            let child_size = node_size(child_node, state.size_mode);
            if let Some(rect) =
                child_highlight_rect(current_rect, parent_size, child_size, child_id)
            {
                overlays.push(rect);
                current_rect = rect;
                parent_id = child_id;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    overlays
}

fn child_highlight_rect(
    bounds: Rect,
    parent_size: u64,
    child_size: u64,
    child_id: usize,
) -> Option<Rect> {
    if bounds.width == 0 || bounds.height == 0 || child_size == 0 || parent_size == 0 {
        return None;
    }

    let highlight_size = child_size.min(parent_size);
    let remainder = parent_size.saturating_sub(highlight_size);

    let mut nodes = vec![TreemapNode {
        node_id: child_id,
        name: String::new(),
        size: highlight_size,
        is_directory: true,
    }];

    if remainder > 0 {
        nodes.push(TreemapNode {
            node_id: usize::MAX,
            name: String::new(),
            size: remainder,
            is_directory: false,
        });
    }

    let tiles = squarified_treemap(&nodes, bounds, nodes.len().max(1));
    tiles
        .into_iter()
        .find(|tile| tile.node.node_id == child_id)
        .map(|tile| tile.rect)
}

fn node_size(node: &TreeNode, mode: SizeDisplayMode) -> u64 {
    match mode {
        SizeDisplayMode::Apparent => node.size,
        SizeDisplayMode::Disk => node.disk_size,
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
        traverse(tree, tree.root(), 0, &mut rows);
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

    #[test]
    fn deep_selection_highlight_overlays_for_nested_entry() {
        use crate::treemap::squarified_treemap;

        let mut state =
            crate::state::AppState::new(PathBuf::from("/root"), crate::config::SortMode::SizeDesc);
        let top = state.tree.add_child(
            0,
            TreeNode::new(PathBuf::from("/root/top"), NodeType::Directory),
        );
        let deep = state.tree.add_child(
            top,
            TreeNode::new(PathBuf::from("/root/top/deep"), NodeType::File),
        );

        if let Some(node) = state.tree.node_mut(top) {
            node.size = 10;
        }
        if let Some(node) = state.tree.node_mut(deep) {
            node.size = 3;
        }

        state.selection = Some(deep);
        state.refresh_treemap_nodes();

        let tiles = squarified_treemap(&state.treemap_nodes, Rect::new(0, 0, 80, 20), 200);
        let path = selection_path(&state);
        let overlays = selection_highlight_overlays(&tiles, &path, &state);

        assert!(!overlays.is_empty());
    }
}
