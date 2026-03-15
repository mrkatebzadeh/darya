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
    pub activity: Rect,
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

    let center = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(vertical[1]);

    let main_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(center[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(center[1]);

    LayoutRegions {
        header: vertical[0],
        tree: main_row[0],
        treemap: main_row[1],
        details: bottom[0],
        activity: bottom[1],
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
        assert_eq!(regions.tree.height, 14);
        assert_eq!(regions.tree.width, 60);
        assert_eq!(regions.treemap.width, 20);
        assert_eq!(regions.details.width, 40);
        assert_eq!(regions.activity.width, 40);
        assert_eq!(regions.details.height, 4);
        assert_eq!(regions.activity.height, 4);
        assert_eq!(regions.tree.y, regions.header.height);
        assert_eq!(regions.footer.y, area.height - regions.footer.height);
    }
}
