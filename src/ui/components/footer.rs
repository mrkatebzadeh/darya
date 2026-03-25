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

const FOOTER_KEYBINDINGS: [(&str, &str); 12] = [
    ("R", "scan"),
    ("?", "help"),
    ("j/k", "move"),
    ("h/l", "collapse/expand"),
    ("enter/tab", "open"),
    ("/", "filter"),
    ("c", "clear"),
    ("b", "size"),
    ("s", "sort"),
    ("t", "treemap"),
    ("r", "scan"),
    ("q", "quit"),
];

pub fn draw_footer_panel(frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
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

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(2);
    let status_trimmed = trim_to_width(&progress_label, area.width as usize);
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

    let mut selected_count = 0;
    let mut chunk_width = 0;
    for candidate in (1..=FOOTER_KEYBINDINGS.len()).rev() {
        let chunk = width / candidate;
        if chunk == 0 {
            continue;
        }
        let max_len = FOOTER_KEYBINDINGS[..candidate]
            .iter()
            .map(|(key, action)| key.len() + 1 + action.len())
            .max()
            .unwrap_or(0);
        if max_len <= chunk {
            selected_count = candidate;
            chunk_width = chunk;
            break;
        }
    }

    if selected_count == 0 {
        return None;
    }

    let remainder = width - (chunk_width * selected_count);
    let mut spans = Vec::new();
    for (idx, &(key, action)) in FOOTER_KEYBINDINGS.iter().take(selected_count).enumerate() {
        let chunk = if idx == selected_count - 1 {
            chunk_width + remainder
        } else {
            chunk_width
        };
        let entry_len = key.len() + 1 + action.len();
        let pad_total = chunk.saturating_sub(entry_len);
        let pad_left = pad_total / 2;
        let pad_right = pad_total - pad_left;

        if pad_left > 0 {
            spans.push(Span::raw(" ".repeat(pad_left)));
        }
        spans.push(Span::styled(
            key,
            Style::default().fg(theme.tile_color(idx)),
        ));
        spans.push(Span::styled(":", Style::default().fg(theme.foreground)));
        spans.push(Span::styled(action, Style::default().fg(theme.foreground)));
        if pad_right > 0 {
            spans.push(Span::raw(" ".repeat(pad_right)));
        }
    }

    Some(Line::from(spans))
}
