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
use crate::state::ScanState;
use crate::theme::Theme;
use crate::ui::helpers::{spinner_symbol, trim_to_width};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::widgets::Paragraph;

pub fn draw_footer_panel(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let base_status = state.status_message.as_deref().unwrap_or("ready");
    let status_text = if matches!(state.scan_state, ScanState::Running(_)) {
        ""
    } else {
        base_status
    };
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
        Paragraph::new(" ").style(Style::default().bg(Color::Reset)),
        area,
    );
    if area.height == 0 {
        return;
    }

    let status = if status_text.is_empty() || status_text == progress_label {
        progress_label.clone()
    } else {
        format!("{status_text} | {progress_label}")
    };
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
            Style::default().fg(theme.foreground).bg(Color::Reset),
        );
    }

    if area.width > hint_width {
        let hint_x = area.x + area.width.saturating_sub(hint_width);
        buf.set_stringn(
            hint_x,
            area.y,
            hint,
            hint_width as usize,
            Style::default().fg(theme.directory).bg(Color::Reset),
        );
    }
}
