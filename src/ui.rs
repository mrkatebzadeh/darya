use crate::{
    layout::LayoutRegions,
    state::{AppState, ScanState},
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
                format!("scanned {} err {}", progress.scanned, progress.errors)
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
        let rows = build_rows(&state.tree, state.selection, theme);
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
                format!("scanned {} err {}", progress.scanned, progress.errors)
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
            Line::from(Span::raw(
                "hjkl: move │ gg/G: jump │ q: quit │ enter: expand",
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
    kind: NodeType,
}

fn build_rows(tree: &FileTree, selection: Option<usize>, theme: Theme) -> Vec<Row<'static>> {
    let mut rows = Vec::new();
    traverse(tree, tree.root(), 0, &mut rows);
    let max_size = rows.iter().map(|row| row.size).max().unwrap_or(1);

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
                Cell::from(format_size(row.size)),
                Cell::from(draw_bar(row.size, max_size, 12)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{NodeType, TreeNode};
    use std::path::PathBuf;

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
}
