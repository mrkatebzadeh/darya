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
use crate::theme::Theme;
use crate::ui::helpers::{ColumnWidths, build_row, chosen_size, collect_tree_rows};
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::terminal::Frame;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Table};

pub fn draw_filesystem_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &mut AppState,
    theme: Theme,
) {
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

    let block = Block::default().borders(Borders::ALL).title("Filesystem");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let overlay_height = 3u16;
    let mut table_area = inner;
    let mut filter_area = None;
    if state.filter_prompt_active && inner.height > overlay_height && inner.width >= 4 {
        filter_area = Some(Rect::new(inner.x, inner.y, inner.width, overlay_height));
        table_area = Rect::new(
            inner.x,
            inner.y + overlay_height,
            inner.width,
            inner.height - overlay_height,
        );
    }

    let table = Table::new(table_rows)
        .widths(&[
            Constraint::Percentage(53),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
        ])
        .column_spacing(1);

    frame.render_widget(table, table_area);

    if let Some(filter_area) = filter_area {
        frame.render_widget(Clear, filter_area);
        let display_line = if state.filter_query.is_empty() {
            Line::from(" ")
        } else {
            Line::from(state.filter_query.clone())
        };
        let filter_box = Paragraph::new(display_line)
            .block(Block::default().title("Filter").borders(Borders::ALL))
            .style(Style::default().fg(theme.foreground).bg(theme.background));
        frame.render_widget(filter_box, filter_area);
    }
}
