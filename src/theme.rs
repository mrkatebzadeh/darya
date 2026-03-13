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
