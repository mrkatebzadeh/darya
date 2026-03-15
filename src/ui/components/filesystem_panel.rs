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

use crate::theme::Theme;
use crate::ui::view_model::FilesystemViewModel;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Style;
use ratatui::terminal::Frame;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Table};

pub fn draw_filesystem_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    view_model: FilesystemViewModel,
    theme: Theme,
) {
    let block = Block::default().borders(Borders::ALL).title("Filesystem");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    const OVERLAY_HEIGHT: u16 = 3;
    let mut table_area = inner;
    let mut filter_area = None;
    if view_model.filter_prompt.is_some() && inner.height > OVERLAY_HEIGHT && inner.width >= 4 {
        filter_area = Some(Rect::new(inner.x, inner.y, inner.width, OVERLAY_HEIGHT));
        table_area = Rect::new(
            inner.x,
            inner.y + OVERLAY_HEIGHT,
            inner.width,
            inner.height - OVERLAY_HEIGHT,
        );
    }

    let table = Table::new(view_model.table_rows)
        .widths(&[
            Constraint::Percentage(53),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
        ])
        .column_spacing(1);

    frame.render_widget(table, table_area);

    if let Some(filter_area) = filter_area {
        frame.render_widget(Clear, filter_area);
        let display_line = Line::from(view_model.filter_prompt.unwrap_or_else(|| " ".into()));
        let filter_box = Paragraph::new(display_line)
            .block(Block::default().title("Filter").borders(Borders::ALL))
            .style(Style::default().fg(theme.foreground).bg(theme.background));
        frame.render_widget(filter_box, filter_area);
    }
}
