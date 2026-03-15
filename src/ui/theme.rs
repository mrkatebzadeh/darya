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

pub const TILE_PALETTE_SIZE: usize = 20;

/// Color palette for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub selection: Color,
    pub directory: Color,
    pub file: Color,
    pub bar: Color,
    pub tile_palette: [Color; TILE_PALETTE_SIZE],
}

impl Theme {
    pub fn tile_color(&self, index: usize) -> Color {
        let mut current = index % TILE_PALETTE_SIZE;
        for _ in 0..TILE_PALETTE_SIZE {
            let color = self.tile_palette[current];
            if color != self.selection {
                return color;
            }
            current = (current + 1) % TILE_PALETTE_SIZE;
        }
        self.bar
    }
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
            tile_palette: [
                Color::LightGreen,
                Color::LightYellow,
                Color::LightBlue,
                Color::LightMagenta,
                Color::LightCyan,
                Color::Yellow,
                Color::LightRed,
                Color::Green,
                Color::Magenta,
                Color::Cyan,
                Color::Red,
                Color::White,
                Color::Gray,
                Color::DarkGray,
                Color::Black,
                Color::Rgb(255, 165, 0),
                Color::Rgb(128, 0, 128),
                Color::Rgb(0, 128, 128),
                Color::Rgb(128, 128, 0),
                Color::Rgb(255, 192, 203),
            ],
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
        assert_eq!(theme.tile_palette.len(), TILE_PALETTE_SIZE);
        assert_eq!(theme.tile_color(0), Color::LightGreen);
        assert_eq!(theme.tile_color(5), Color::Yellow);
        assert_eq!(theme.tile_color(19), Color::Rgb(255, 192, 203));
    }
}
