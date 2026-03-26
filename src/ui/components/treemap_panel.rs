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

use crate::state::AppState;
use crate::theme::Theme;
use crate::treemap::{TreemapLayout, TreemapNode, contextual_treemap_layout};
use crate::ui::helpers::{draw_treemap_tile, fill_rect, gather_child_nodes, selection_path};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn draw_treemap_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    theme: Theme,
    cache: &mut TreemapLayoutCache,
) {
    let panel_block = Block::default()
        .borders(Borders::ALL)
        .title("Treemap")
        .style(Style::default().bg(Color::Reset));
    let inner = panel_block.inner(area);
    frame.render_widget(panel_block, area);

    if inner.width < 2 || inner.height < 2 {
        return;
    }

    let max_tiles_for_panel = usize::from(inner.width) * usize::from(inner.height) / 2;
    let max_tiles = 200_usize.min(max_tiles_for_panel.max(1));
    let path = selection_path(state);
    let layout = cache.layout_for(
        inner,
        &path,
        state.treemap_revision,
        &state.ui.treemap_nodes,
        max_tiles,
        |parent, limit| gather_child_nodes(parent, state, limit),
    );
    if layout.tiles.is_empty() {
        frame.render_widget(
            Paragraph::new("No sized children")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.foreground).bg(Color::Reset)),
            inner,
        );
        return;
    }

    for tile in &layout.tiles {
        draw_treemap_tile(frame, tile, theme);
    }

    if let Some(selection_rect) = state
        .navigation
        .selection
        .and_then(|selection| layout.node_rects.get(&selection).copied())
    {
        fill_rect(frame, selection_rect, theme.selection, theme.background);
    }
}

pub struct TreemapLayoutCache {
    key: Option<TreemapLayoutKey>,
    layout: Option<TreemapLayout>,
}

impl TreemapLayoutCache {
    pub fn new() -> Self {
        Self {
            key: None,
            layout: None,
        }
    }

    pub(crate) fn layout_for<F>(
        &mut self,
        bounds: Rect,
        selection_path: &crate::ui::helpers::SelectionPath,
        revision: u64,
        root_nodes: &[TreemapNode],
        max_nodes: usize,
        child_provider: F,
    ) -> &TreemapLayout
    where
        F: FnMut(usize, usize) -> Vec<TreemapNode>,
    {
        let provider = child_provider;
        let key = TreemapLayoutKey {
            bounds,
            revision,
            selection_path: selection_path.clone(),
        };
        if self.key.as_ref() == Some(&key) {
            return self.layout.as_ref().unwrap();
        }
        let layout = contextual_treemap_layout(
            root_nodes,
            bounds,
            selection_path.as_slice(),
            max_nodes,
            provider,
        );
        self.key = Some(key);
        self.layout = Some(layout);
        self.layout.as_ref().unwrap()
    }
}

impl Default for TreemapLayoutCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreemapLayoutKey {
    bounds: Rect,
    revision: u64,
    selection_path: crate::ui::helpers::SelectionPath,
}
