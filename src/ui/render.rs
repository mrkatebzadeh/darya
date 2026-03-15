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

use crate::state::{AppState, ScanState};
use crate::theme::Theme;
use crate::treemap::{TreemapLayout, TreemapNode, TreemapTile, contextual_treemap_layout};
use crate::ui::{helpers::*, layout::LayoutRegions};
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::Style;
use ratatui::terminal::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Table};

/// Renderer responsible for drawing the main UI panels.
#[derive(Default)]
pub struct Ui {
    treemap_cache: TreemapLayoutCache,
}

impl Ui {
    pub fn draw(
        &mut self,
        frame: &mut Frame<'_>,
        layout: LayoutRegions,
        state: &mut AppState,
        theme: Theme,
    ) {
        self.draw_header(frame, layout.header, state, theme);
        self.draw_tree(frame, layout.tree, state, theme);
        self.draw_treemap(frame, layout.treemap, state, theme);
        self.draw_details(frame, layout.details, state, theme);
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

        let header = Paragraph::new(Line::from(vec![
            Span::styled("root: ", Style::default().fg(theme.directory)),
            Span::styled(root_label, Style::default().fg(theme.foreground)),
            Span::raw(format!(" | sort:{} ", sort_mode_label(state.sort_mode))),
        ]))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().bg(theme.background));

        frame.render_widget(header, area);
    }

    fn draw_tree(&self, frame: &mut Frame<'_>, area: Rect, state: &mut AppState, theme: Theme) {
        let tree_rows = collect_tree_rows(
            &state.tree,
            &state.filter_query,
            state.filter_active,
            state.display_options,
        );
        let max_size = tree_rows
            .iter()
            .map(|row| chosen_size(row, state.size_mode, state.display_options))
            .max()
            .unwrap_or(1);

        let visible_height = area.height.saturating_sub(2) as usize;
        if tree_rows.is_empty() {
            state.set_scroll_offset(0);
        }

        let mut offset = state.scroll_offset;
        if !tree_rows.is_empty() && visible_height > 0 {
            let selected_index = tree_rows
                .iter()
                .position(|row| Some(row.id()) == state.selection)
                .unwrap_or(0);
            if selected_index < offset {
                offset = selected_index;
            } else if selected_index >= offset + visible_height {
                offset = selected_index + 1 - visible_height;
            }
            let max_offset = tree_rows.len().saturating_sub(visible_height);
            if offset > max_offset {
                offset = max_offset;
            }
        } else {
            offset = 0;
        }
        state.set_scroll_offset(offset);

        let percent_column_width = (((area.width as usize) * 30) / 100).max(1);
        let size_column_width = (((area.width as usize) * 15) / 100).max(1);
        let table_rows = if visible_height == 0 {
            Vec::new()
        } else {
            tree_rows
                .iter()
                .skip(offset)
                .take(visible_height)
                .map(|row| {
                    build_row(
                        row,
                        state.selection,
                        theme,
                        state.size_mode,
                        max_size,
                        state.display_options,
                        ColumnWidths {
                            percent: percent_column_width,
                            size: size_column_width,
                        },
                    )
                })
                .collect()
        };

        let table = Table::new(table_rows)
            .block(Block::default().borders(Borders::ALL).title("Filesystem"))
            .widths(&[
                Constraint::Percentage(53),
                Constraint::Percentage(30),
                Constraint::Percentage(15),
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

        frame.render_widget(
            Paragraph::new(" ").style(Style::default().bg(theme.background)),
            area,
        );
        if area.height == 0 {
            return;
        }

        let status = format!("{status_text} | {progress_label}");
        let hint = "Press ? for keybindings";
        let hint_width = hint.len() as u16;
        let status_width = area.width.saturating_sub(hint_width + 1);
        let status_trimmed = trim_to_width(&status, status_width as usize);

        let buf = frame.buffer_mut();
        if status_width > 0 {
            buf.set_stringn(
                area.x,
                area.y,
                &status_trimmed,
                status_width as usize,
                Style::default().fg(theme.foreground).bg(theme.background),
            );
        }

        if area.width > hint_width {
            let hint_x = area.x + area.width.saturating_sub(hint_width);
            buf.set_stringn(
                hint_x,
                area.y,
                hint,
                hint_width as usize,
                Style::default().fg(theme.directory).bg(theme.background),
            );
        }
    }

    fn draw_treemap(&mut self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        let panel = Block::default().borders(Borders::ALL).title("Treemap");
        let inner = panel.inner(area);
        frame.render_widget(panel, area);

        if inner.width < 2 || inner.height < 2 {
            return;
        }

        let max_tiles_for_panel = usize::from(inner.width) * usize::from(inner.height) / 2;
        let max_tiles = 200_usize.min(max_tiles_for_panel.max(1));
        let path = selection_path(state);
        let layout = self.treemap_cache.layout_for(
            inner,
            &path,
            state.treemap_revision,
            &state.treemap_nodes,
            max_tiles,
            |parent, limit| gather_child_nodes(parent, state, limit),
        );
        if layout.tiles.is_empty() {
            frame.render_widget(
                Paragraph::new("No sized children")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.foreground).bg(theme.background)),
                inner,
            );
            return;
        }

        for tile in &layout.tiles {
            draw_treemap_tile(frame, tile, theme);
        }

        if let Some(selection_rect) = state
            .selection
            .and_then(|selection| layout.node_rects.get(&selection).copied())
        {
            fill_rect(frame, selection_rect, theme.selection, theme.background);
        }
    }

    fn draw_details(&self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        frame.render_widget(Clear, area);
        let block = Block::default().borders(Borders::ALL).title("Details");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        frame.render_widget(Clear, inner);
        let lines = detail_panel_lines(state);

        let glyph_col_width: u16 = 3;
        let key_col_width: u16 = 13;
        let value_start = glyph_col_width.saturating_add(key_col_width);
        if inner.width <= value_start {
            return;
        }
        let value_col_width = inner.width - value_start;

        let buf = frame.buffer_mut();
        for (idx, raw_line) in lines.iter().take(inner.height as usize).enumerate() {
            let y = inner.y + idx as u16;
            let (glyph, key, value) = split_detail_line(raw_line);

            buf.set_stringn(
                inner.x,
                y,
                glyph,
                glyph_col_width as usize,
                Style::default().fg(theme.selection),
            );
            buf.set_stringn(
                inner.x + glyph_col_width,
                y,
                key,
                key_col_width as usize,
                Style::default().fg(theme.directory),
            );
            buf.set_stringn(
                inner.x + value_start,
                y,
                value,
                value_col_width as usize,
                Style::default().fg(theme.foreground),
            );
        }
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
            Line::from("  b: size mode, s: sort mode, r: rescan, R: start scan"),
            Line::from("  E/I: export/import snapshot"),
            Line::from("  H: toggle hidden files"),
            Line::from("  ?: toggle this help, q: quit"),
        ];

        let popup = Paragraph::new(lines)
            .block(Block::default().title("DAR Help").borders(Borders::ALL))
            .style(Style::default().fg(theme.foreground).bg(theme.background));

        frame.render_widget(Clear, area);
        frame.render_widget(popup, area);
    }
}

fn split_detail_line(line: &str) -> (&str, &str, &str) {
    let mut parts = line.splitn(3, '\t');
    let glyph = parts.next().unwrap_or("");
    let key = parts.next().unwrap_or("");
    let value = parts.next().unwrap_or("");
    (glyph, key, value)
}

fn draw_treemap_tile(frame: &mut Frame<'_>, tile: &TreemapTile, theme: Theme) {
    if tile.rect.width == 0 || tile.rect.height == 0 {
        return;
    }

    let color = theme.tile_color(tile.color_index);
    fill_rect(frame, tile.rect, color, theme.background);
}

#[derive(Default)]
struct TreemapLayoutCache {
    key: Option<TreemapLayoutKey>,
    layout: Option<TreemapLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreemapLayoutKey {
    bounds: Rect,
    revision: u64,
    selection_path: Vec<usize>,
}

impl TreemapLayoutCache {
    fn layout_for<F>(
        &mut self,
        bounds: Rect,
        selection_path: &[usize],
        revision: u64,
        root_nodes: &[TreemapNode],
        max_nodes: usize,
        child_provider: F,
    ) -> &TreemapLayout
    where
        F: FnMut(usize, usize) -> Vec<TreemapNode>,
    {
        let provider = child_provider;
        let key = TreemapLayoutKey {
            bounds,
            revision,
            selection_path: selection_path.to_vec(),
        };

        if self.key.as_ref() == Some(&key) {
            return self.layout.as_ref().unwrap();
        }

        let layout =
            contextual_treemap_layout(root_nodes, bounds, selection_path, max_nodes, provider);

        self.key = Some(key);
        self.layout = Some(layout);
        self.layout.as_ref().unwrap()
    }
}
