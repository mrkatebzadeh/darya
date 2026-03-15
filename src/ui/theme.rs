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
use serde::{Deserialize, Serialize};

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
    pub bar_bg: Color,
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
            background: Color::Rgb(30, 30, 46),    // frappe base
            foreground: Color::Rgb(198, 208, 245), // text
            selection: Color::Rgb(140, 170, 238),  // overlay
            directory: Color::Rgb(180, 184, 254),  // lavender
            file: Color::Rgb(198, 208, 245),
            bar: Color::Rgb(137, 180, 250), // sky
            bar_bg: Color::Rgb(92, 108, 124),
            tile_palette: [
                Color::Rgb(245, 224, 220), // rosewater
                Color::Rgb(242, 205, 205), // flamingo
                Color::Rgb(245, 194, 231), // pink
                Color::Rgb(203, 166, 247), // mauve
                Color::Rgb(243, 139, 168), // red
                Color::Rgb(235, 160, 172), // maroon
                Color::Rgb(250, 179, 135), // peach
                Color::Rgb(249, 226, 175), // yellow
                Color::Rgb(166, 227, 161), // green
                Color::Rgb(148, 226, 213), // teal
                Color::Rgb(137, 180, 250), // sky
                Color::Rgb(116, 199, 236), // sapphire
                Color::Rgb(177, 190, 254), // lavender
                Color::Rgb(146, 199, 248),
                Color::Rgb(185, 123, 255),
                Color::Rgb(255, 121, 198),
                Color::Rgb(255, 195, 160),
                Color::Rgb(255, 255, 255),
                Color::Rgb(92, 108, 124),
                Color::Rgb(17, 17, 27),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(with = "color")]
    pub background: Color,
    #[serde(with = "color")]
    pub foreground: Color,
    #[serde(with = "color")]
    pub selection: Color,
    #[serde(with = "color")]
    pub directory: Color,
    #[serde(with = "color")]
    pub file: Color,
    #[serde(with = "color")]
    pub bar: Color,
    #[serde(with = "color")]
    pub bar_bg: Color,
    #[serde(with = "color_vec")]
    pub tile_palette: Vec<Color>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        let theme = Theme::default();
        Self {
            background: theme.background,
            foreground: theme.foreground,
            selection: theme.selection,
            directory: theme.directory,
            file: theme.file,
            bar: theme.bar,
            bar_bg: theme.bar_bg,
            tile_palette: theme.tile_palette.to_vec(),
        }
    }
}

impl ThemeConfig {
    pub fn to_theme(&self) -> Theme {
        let mut palette = [Color::Reset; TILE_PALETTE_SIZE];
        for (i, color) in self.tile_palette.iter().enumerate().take(TILE_PALETTE_SIZE) {
            palette[i] = *color;
        }
        if self.tile_palette.len() < TILE_PALETTE_SIZE {
            let default = Theme::default();
            for (i, color) in default
                .tile_palette
                .iter()
                .enumerate()
                .skip(self.tile_palette.len())
            {
                palette[i] = *color;
            }
        }
        Theme {
            background: self.background,
            foreground: self.foreground,
            selection: self.selection,
            directory: self.directory,
            file: self.file,
            bar: self.bar,
            bar_bg: self.bar_bg,
            tile_palette: palette,
        }
    }
}

pub mod color {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&color_to_string(color))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        parse_color(&text).map_err(serde::de::Error::custom)
    }
}

mod color_vec {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(colors: &[Color], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strs: Vec<String> = colors.iter().map(color_to_string).collect();
        strs.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Color>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let texts = Vec::<String>::deserialize(deserializer)?;
        texts
            .into_iter()
            .map(|text| parse_color(&text).map_err(serde::de::Error::custom))
            .collect()
    }
}

fn color_to_string(color: &Color) -> String {
    match color {
        Color::Reset => "Reset".to_string(),
        Color::Black => "Black".to_string(),
        Color::Red => "Red".to_string(),
        Color::Green => "Green".to_string(),
        Color::Yellow => "Yellow".to_string(),
        Color::Blue => "Blue".to_string(),
        Color::Magenta => "Magenta".to_string(),
        Color::Cyan => "Cyan".to_string(),
        Color::Gray => "Gray".to_string(),
        Color::DarkGray => "DarkGray".to_string(),
        Color::LightRed => "LightRed".to_string(),
        Color::LightGreen => "LightGreen".to_string(),
        Color::LightYellow => "LightYellow".to_string(),
        Color::LightBlue => "LightBlue".to_string(),
        Color::LightMagenta => "LightMagenta".to_string(),
        Color::LightCyan => "LightCyan".to_string(),
        Color::White => "White".to_string(),
        Color::Rgb(r, g, b) => format!("rgb({},{},{})", r, g, b),
        Color::Indexed(index) => format!("indexed({})", index),
    }
}

fn parse_color(value: &str) -> Result<Color, String> {
    let normalized = value.trim();
    if let Some(idx) = normalized.strip_prefix("rgb(")
        && let Some(rest) = idx.strip_suffix(')')
    {
        let parts: Vec<&str> = rest.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            let r = parts[0]
                .parse::<u8>()
                .map_err(|_| format!("invalid rgb component {}", parts[0]))?;
            let g = parts[1]
                .parse::<u8>()
                .map_err(|_| format!("invalid rgb component {}", parts[1]))?;
            let b = parts[2]
                .parse::<u8>()
                .map_err(|_| format!("invalid rgb component {}", parts[2]))?;
            return Ok(Color::Rgb(r, g, b));
        }
    }
    if let Some(idx) = normalized.strip_prefix("indexed(")
        && let Some(rest) = idx.strip_suffix(')')
    {
        let value = rest
            .trim()
            .parse::<u8>()
            .map_err(|_| format!("invalid index {}", rest))?;
        return Ok(Color::Indexed(value));
    }
    match normalized.to_lowercase().as_str() {
        "reset" => Ok(Color::Reset),
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "gray" => Ok(Color::Gray),
        "darkgray" => Ok(Color::DarkGray),
        "lightred" => Ok(Color::LightRed),
        "lightgreen" => Ok(Color::LightGreen),
        "lightyellow" => Ok(Color::LightYellow),
        "lightblue" => Ok(Color::LightBlue),
        "lightmagenta" => Ok(Color::LightMagenta),
        "lightcyan" => Ok(Color::LightCyan),
        "white" => Ok(Color::White),
        other => Err(format!("unknown color '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_is_expected() {
        let theme = Theme::default();
        assert_eq!(theme.background, Color::Rgb(30, 30, 46));
        assert_eq!(theme.foreground, Color::Rgb(198, 208, 245));
        assert_eq!(theme.selection, Color::Rgb(140, 170, 238));
        assert_eq!(theme.directory, Color::Rgb(180, 184, 254));
        assert_eq!(theme.file, Color::Rgb(198, 208, 245));
        assert_eq!(theme.bar, Color::Rgb(137, 180, 250));
        assert_eq!(theme.bar_bg, Color::Rgb(92, 108, 124));
        assert_eq!(theme.tile_palette.len(), TILE_PALETTE_SIZE);
        assert_eq!(theme.tile_color(0), Color::Rgb(245, 224, 220));
        assert_eq!(theme.tile_color(5), Color::Rgb(235, 160, 172));
        assert_eq!(theme.tile_color(19), Color::Rgb(17, 17, 27));
    }
}
