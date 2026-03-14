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

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Named layout regions that divide the main screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRegions {
    pub header: Rect,
    pub tree: Rect,
    pub treemap: Rect,
    pub details: Rect,
    pub footer: Rect,
}

/// Split the available `area` into header, body panels, and footer regions.
pub fn split_layout(area: Rect) -> LayoutRegions {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(vertical[1]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(body[1]);

    LayoutRegions {
        header: vertical[0],
        tree: body[0],
        treemap: right[0],
        details: right[1],
        footer: vertical[2],
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
        assert_eq!(regions.footer.height, 3);
        assert_eq!(regions.tree.height, 18);
        assert_eq!(regions.tree.width, 56);
        assert_eq!(regions.treemap.width, 24);
        assert_eq!(regions.details.width, 24);
        assert_eq!(
            regions.treemap.height + regions.details.height,
            regions.tree.height
        );
        assert_eq!(regions.tree.y, regions.header.height);
        assert_eq!(regions.footer.y, area.height - regions.footer.height);
    }
}
