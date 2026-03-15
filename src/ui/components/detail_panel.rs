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
use crate::ui::view_model::DetailViewModel;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::widgets::{Block, Borders, Clear};

pub fn draw_detail_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    view_model: &DetailViewModel,
    theme: Theme,
) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Details")
        .style(Style::default().bg(Color::Reset));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    frame.render_widget(Clear, inner);
    let glyph_col_width: u16 = 3;
    let key_col_width: u16 = 13;
    let value_start = glyph_col_width + key_col_width;
    if inner.width <= value_start {
        return;
    }
    let value_col_width = inner.width - value_start;

    let buf = frame.buffer_mut();
    for (idx, entry) in view_model.entries.iter().enumerate() {
        if idx as u16 >= inner.height {
            break;
        }
        let y = inner.y + idx as u16;
        let glyph = &entry.glyph;
        let key = &entry.key;
        let value = if value_col_width > 0 {
            trim_to_width(&entry.value, value_col_width as usize)
        } else {
            String::new()
        };

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
            &value,
            value_col_width as usize,
            Style::default().fg(theme.foreground),
        );
    }
}
