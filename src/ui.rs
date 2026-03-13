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
    layout::LayoutRegions,
    state::{AppState, ScanState, SizeDisplayMode},
    theme::Theme,
    tree::{FileTree, NodeType},
};
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    terminal::Frame,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
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
        self.draw_footer(frame, layout.footer, state, theme);
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
            Span::raw(" "),
            Span::styled(progress_label, Style::default().fg(theme.selection)),
        ]))
        .block(Block::default().borders(Borders::ALL).title("dar"))
        .style(Style::default().bg(theme.background));

        frame.render_widget(header, area);
    }

    fn draw_tree(&self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        let rows = build_rows(&state.tree, state.selection, theme, state.size_mode);
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
                "hjkl: move │ gg/G: jump │ enter/tab: toggle │ d: delete │ o: open │ r: rescan │ b: size mode │ E/I: export/import │ q: quit",
            )),
        ])
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().bg(theme.background));

        frame.render_widget(footer, area);
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
) -> Vec<Row<'static>> {
    let mut rows = Vec::new();
    traverse(tree, tree.root(), 0, &mut rows);
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
}
