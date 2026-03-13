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
    pub footer: Rect,
}

/// Split the available `area` into header, tree, and footer regions.
pub fn split_layout(area: Rect) -> LayoutRegions {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
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
        assert_eq!(regions.footer.height, 3);
        assert_eq!(regions.tree.height, 18);
        assert_eq!(regions.tree.y, regions.header.height);
        assert_eq!(regions.footer.y, area.height - regions.footer.height);
    }
}
