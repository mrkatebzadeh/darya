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
use crate::ui::{
    components,
    helpers::*,
    layout::LayoutRegions,
    view_model::{ActivityViewModel, DetailViewModel, FilesystemViewModel},
};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::terminal::Frame;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

/// Renderer responsible for drawing the main UI panels.
struct TreeRowsCache {
    revision: u64,
    rows: Vec<TreeRow>,
}

impl TreeRowsCache {
    fn new(revision: u64, rows: Vec<TreeRow>) -> Self {
        Self { revision, rows }
    }
}

#[derive(Default)]
pub struct Ui {
    treemap_cache: components::TreemapLayoutCache,
    tree_rows_cache: Option<TreeRowsCache>,
}

impl Ui {
    pub fn draw(
        &mut self,
        frame: &mut Frame<'_>,
        layout: LayoutRegions,
        state: &mut AppState,
        theme: Theme,
    ) {
        self.draw_header(frame, layout.header, state, theme);
        let tree_rows = self.cached_tree_rows(state);
        let filesystem_vm = FilesystemViewModel::build(state, layout.tree, theme, tree_rows);
        components::draw_filesystem_panel(frame, layout.tree, filesystem_vm, theme);
        if state.treemap_visible && layout.treemap.width > 0 && layout.treemap.height > 0 {
            components::draw_treemap_panel(
                frame,
                layout.treemap,
                state,
                theme,
                &mut self.treemap_cache,
            );
        }
        let detail_vm = DetailViewModel::build(state);
        components::draw_detail_panel(frame, layout.details, &detail_vm, theme);
        let activity_vm = ActivityViewModel::build(state);
        components::draw_activity_panel(frame, layout.activity, &activity_vm, theme);
        components::draw_footer_panel(frame, layout.footer, state, theme);
        if state.show_help {
            self.draw_help_modal(frame, state, theme);
        }
    }

    fn cached_tree_rows(&mut self, state: &AppState) -> &[TreeRow] {
        let revision = state.ui_revision();
        let needs_refresh = self
            .tree_rows_cache
            .as_ref()
            .map(|cache| cache.revision != revision)
            .unwrap_or(true);

        if needs_refresh {
            let rows = collect_tree_rows(
                &state.tree,
                &state.filter_query,
                state.filter_active,
                state.display_options,
            );
            self.tree_rows_cache = Some(TreeRowsCache::new(revision, rows));
        }

        &self.tree_rows_cache.as_ref().unwrap().rows
    }

    fn draw_header(&self, frame: &mut Frame<'_>, area: Rect, state: &AppState, theme: Theme) {
        let header = Paragraph::new(Line::from(vec![
            Span::styled("Sort: ", Style::default().fg(theme.directory)),
            Span::styled(
                sort_mode_label(state.sort_mode),
                Style::default().fg(theme.foreground),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().bg(Color::Reset));

        frame.render_widget(header, area);
    }

    fn draw_help_modal(&self, frame: &mut Frame<'_>, _state: &AppState, theme: Theme) {
        let frame_area = frame.size();
        let mut width = frame_area.width.min(80);
        let min_width = 40u16;
        width = width.max(min_width).min(frame_area.width);
        let computed_height = frame_area.height as u32;
        let computed_height = computed_height.saturating_mul(70) / 100;
        let mut height = u16::try_from(computed_height).unwrap_or(u16::MAX);
        let min_height = 10u16;
        if height < min_height {
            height = min_height;
        }
        height = height.min(frame_area.height);
        if width == 0 || height == 0 {
            return;
        }
        let x = frame_area.x + frame_area.width.saturating_sub(width) / 2;
        let y = frame_area.y + frame_area.height.saturating_sub(height) / 2;
        let area = Rect::new(x, y, width, height);
        let lines = vec![
            Line::from("██████╗  █████╗ ██████╗ ██╗   ██╗ █████╗"),
            Line::from("██╔══██╗██╔══██╗██╔══██╗╚██╗ ██╔╝██╔══██╗"),
            Line::from("██║  ██║███████║██████╦╝ ╚████╔╝ ███████║"),
            Line::from("██║  ██║██╔══██║██╔══██╗  ╚██╔╝  ██╔══██║"),
            Line::from("██████╔╝██║  ██║██║  ██║   ██║   ██║  ██║"),
            Line::from("╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝"),
            Line::from(""),
            Line::from("Keybindings:"),
            Line::from("  j/k or up/down: move"),
            Line::from("  enter/tab: toggle folder"),
            Line::from("  d: delete (with confirmation)"),
            Line::from("  o: open selected path"),
            Line::from("  /: start filter, c: clear filter"),
            Line::from("  b: size mode, s: sort mode, r: rescan, R: start scan"),
            Line::from("  E/I: export/import snapshot"),
            Line::from("  H: toggle hidden files"),
            Line::from("  t: toggle the treemap panel"),
            Line::from("  ?: toggle this help, q: quit"),
        ];

        let popup = Paragraph::new(lines)
            .block(Block::default().title("").borders(Borders::ALL))
            .style(Style::default().fg(theme.foreground).bg(Color::Reset));

        frame.render_widget(Clear, area);
        frame.render_widget(popup, area);
    }
}
