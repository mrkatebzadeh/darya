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

use crate::state::{AppState, ScanState, StatusMessage};
use crate::theme::Theme;
use crate::ui::helpers::{spinner_symbol, trim_to_width};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

const FOOTER_KEYBINDINGS: [(&str, &str); 11] = [
    ("?", "help"),
    ("j/k", "move"),
    ("h/l", "collapse/expand"),
    ("enter/tab", "open"),
    ("/", "filter"),
    ("c", "clear"),
    ("b", "size"),
    ("s", "sort"),
    ("t", "treemap"),
    ("r/R", "scan"),
    ("q", "quit"),
];

pub fn draw_footer_panel(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
    let base_status = state.status_text();
    let status_text = if matches!(state.scan_state, ScanState::Running(_)) {
        ""
    } else {
        base_status.as_str()
    };
    let progress_label = match &state.scan_state {
        ScanState::Running(progress) => {
            let spinner = spinner_symbol(state.spinner_phase);
            format!(
                "{spinner} Scanning... {} entries, {} errors",
                progress.scanned, progress.errors
            )
        }
        ScanState::Error(message) => format!("Error: {message}"),
        ScanState::Completed => StatusMessage::ScanComplete.to_string(),
        _ => "Scan idle".into(),
    };

    if area.height == 0 {
        return;
    }

    let status = if status_text.is_empty() || status_text == progress_label {
        progress_label.clone()
    } else {
        format!("{status_text} | {progress_label}")
    };

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(2);
    let status_trimmed = trim_to_width(&status, area.width as usize);
    lines.push(Line::from(Span::styled(
        status_trimmed,
        Style::default().fg(theme.foreground),
    )));

    if area.height > 1
        && let Some(key_line) = footer_binding_line(area.width as usize, theme)
    {
        lines.push(key_line);
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(Color::Reset));
    frame.render_widget(paragraph, area);
}

fn footer_binding_line(width: usize, theme: Theme) -> Option<Line<'static>> {
    if width == 0 {
        return None;
    }

    let mut remaining = width;
    let mut spans = Vec::new();
    let mut first_entry = true;

    for (idx, &(key, action)) in FOOTER_KEYBINDINGS.iter().enumerate() {
        let entry_len = key.len() + 1 + action.len();
        let required = entry_len + if first_entry { 0 } else { 1 };
        if remaining < required {
            break;
        }

        if !first_entry {
            spans.push(Span::raw(" "));
            remaining -= 1;
        }

        spans.push(Span::styled(
            key,
            Style::default().fg(theme.tile_color(idx)),
        ));
        spans.push(Span::styled(":", Style::default().fg(theme.foreground)));
        spans.push(Span::styled(action, Style::default().fg(theme.foreground)));

        remaining -= entry_len;
        first_entry = false;
    }

    if spans.is_empty() {
        None
    } else {
        Some(Line::from(spans))
    }
}
