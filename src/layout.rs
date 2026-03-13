use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Named layout regions that divide the main screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRegions {
    pub header: Rect,
    pub tree: Rect,
    pub footer: Rect,
}

/// Split the available `area` into header, tree, and footer regions.
pub fn split_layout(area: Rect) -> LayoutRegions {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

    LayoutRegions {
        header: chunks[0],
        tree: chunks[1],
        footer: chunks[2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_splits_evenly() {
        let area = Rect::new(0, 0, 80, 24);
        let regions = split_layout(area);
        assert_eq!(regions.header.height, 3);
        assert_eq!(regions.footer.height, 2);
        assert_eq!(regions.tree.height, 19);
        assert_eq!(regions.tree.y, regions.header.height);
        assert_eq!(regions.footer.y, area.height - regions.footer.height);
    }
}
