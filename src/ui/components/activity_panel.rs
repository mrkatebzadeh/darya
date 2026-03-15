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
use crate::ui::helpers::trim_to_width;
use crate::ui::view_model::ActivityViewModel;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn draw_activity_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    view_model: &ActivityViewModel,
    theme: Theme,
) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Activity")
        .style(Style::default().bg(Color::Reset));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let label_width = 17_u16;
    let value_width = inner.width.saturating_sub(label_width + 1) as usize;
    let mut lines = Vec::new();
    for metric in view_model.metrics.iter().take(inner.height as usize) {
        let trimmed_value = if value_width > 0 {
            trim_to_width(&metric.value, value_width)
        } else {
            String::new()
        };
        let line = Line::from(vec![
            Span::styled(
                format!(
                    "{label:<width$}",
                    label = metric.label,
                    width = label_width as usize
                ),
                Style::default().fg(theme.directory),
            ),
            Span::raw(" "),
            Span::styled(trimmed_value, Style::default().fg(theme.foreground)),
        ]);
        lines.push(line);
    }

    let paragraph =
        Paragraph::new(lines).style(Style::default().fg(theme.foreground).bg(Color::Reset));
    frame.render_widget(paragraph, inner);
}
