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

use ratatui::style::Color;

/// Color palette for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub selection: Color,
    pub directory: Color,
    pub file: Color,
    pub bar: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::White,
            selection: Color::Blue,
            directory: Color::Cyan,
            file: Color::Gray,
            bar: Color::LightGreen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_is_expected() {
        let theme = Theme::default();
        assert_eq!(theme.background, Color::Reset);
        assert_eq!(theme.foreground, Color::White);
        assert_eq!(theme.selection, Color::Blue);
        assert_eq!(theme.directory, Color::Cyan);
        assert_eq!(theme.file, Color::Gray);
        assert_eq!(theme.bar, Color::LightGreen);
    }
}
