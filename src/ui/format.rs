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

pub(crate) fn trim_to_width(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if value.len() <= width {
        value.to_string()
    } else {
        value.chars().take(width).collect()
    }
}

pub(crate) fn percent_bar(percent: f64, width: usize) -> String {
    let ratio = (percent.clamp(0.0, 100.0) / 100.0).min(1.0);
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "-".repeat(empty))
}

pub(crate) fn format_size_custom(bytes: u64, use_si: bool) -> String {
    let div = if use_si { 1000.0 } else { 1024.0 };
    let (kib, mib, gib) = if use_si {
        ("kB", "MB", "GB")
    } else {
        ("KiB", "MiB", "GiB")
    };
    let value = bytes as f64;
    if value >= div * div * div {
        format!("{:.1} {}", value / (div * div * div), gib)
    } else if value >= div * div {
        format!("{:.1} {}", value / (div * div), mib)
    } else if value >= div {
        format!("{:.1} {}", value / div, kib)
    } else {
        format!("{bytes} B")
    }
}
