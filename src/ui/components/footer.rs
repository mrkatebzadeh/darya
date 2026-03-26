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
use crate::ui::format::trim_to_width;
use crate::ui::helpers::spinner_symbol;
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
    let progress_label = match &state.scan.state {
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

    let progress_label = capitalize_first_alpha(&progress_label);

    if area.height == 0 {
        return;
    }

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(2);
    let cap_left = "";
    let cap_right = "";
    let cap_width = cap_left.chars().count() + cap_right.chars().count();
    let inner_width = (area.width as usize).saturating_sub(cap_width);
    let centered_inner = if inner_width == 0 {
        String::new()
    } else {
        let trimmed = trim_to_width(&progress_label, inner_width);
        center_text(&trimmed, inner_width)
    };
    let progress_fg =
        contrast_color(theme.tile_color(0), &theme.tile_palette).unwrap_or(theme.foreground);
    let progress_bg = theme.tile_color(0);
    let mut spans = Vec::new();
    spans.push(Span::styled(
        cap_left,
        Style::default().fg(progress_bg).bg(Color::Reset),
    ));
    if inner_width > 0 {
        spans.push(Span::styled(
            centered_inner,
            Style::default().fg(progress_fg).bg(progress_bg),
        ));
    }
    spans.push(Span::styled(
        cap_right,
        Style::default().fg(progress_bg).bg(Color::Reset),
    ));
    lines.push(Line::from(spans));

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

fn capitalize_first_alpha(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalized = false;
    for ch in text.chars() {
        if !capitalized && ch.is_alphabetic() {
            result.extend(ch.to_uppercase());
            capitalized = true;
        } else {
            result.push(ch);
        }
    }
    result
}

fn center_text(text: &str, width: usize) -> String {
    let text_len = text.chars().count();
    if width <= text_len {
        return text.to_string();
    }
    let total_padding = width - text_len;
    let left_padding = total_padding / 2;
    let right_padding = total_padding - left_padding;
    let mut result = String::with_capacity(width);
    result.extend(std::iter::repeat_n(' ', left_padding));
    result.push_str(text);
    result.extend(std::iter::repeat_n(' ', right_padding));
    result
}

fn contrast_color(bg: Color, palette: &[Color]) -> Option<Color> {
    let bg_lum = color_luminance(bg)?;
    let mut best_ratio = 0.0;
    let mut best_color = None;
    for &candidate in palette {
        if candidate == bg {
            continue;
        }
        if let Some(candidate_lum) = color_luminance(candidate) {
            let ratio = contrast_ratio(bg_lum, candidate_lum);
            if ratio > best_ratio {
                best_ratio = ratio;
                best_color = Some(candidate);
            }
        }
    }
    if best_ratio >= 4.5 { best_color } else { None }
}

fn color_luminance(color: Color) -> Option<f64> {
    match color {
        Color::Rgb(r, g, b) => Some(
            0.2126 * relative_channel(r)
                + 0.7152 * relative_channel(g)
                + 0.0722 * relative_channel(b),
        ),
        _ => None,
    }
}

fn relative_channel(value: u8) -> f64 {
    let c = value as f64 / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn contrast_ratio(a: f64, b: f64) -> f64 {
    let (l1, l2) = if a >= b { (a, b) } else { (b, a) };
    (l1 + 0.05) / (l2 + 0.05)
}
