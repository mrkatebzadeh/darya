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
use crate::ui::helpers::{
    ColumnWidths, build_row, chosen_size, collect_tree_rows, detail_panel_lines,
};
use ratatui::layout::Rect;
use ratatui::widgets::Row;

#[derive(Debug)]
pub struct DetailEntry {
    pub glyph: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct DetailViewModel {
    pub entries: Vec<DetailEntry>,
}

impl DetailViewModel {
    pub fn build(state: &AppState) -> Self {
        let lines = detail_panel_lines(state);
        let entries = lines
            .into_iter()
            .map(|line| {
                let mut parts = line.splitn(3, '\t');
                let glyph = parts.next().unwrap_or_default().to_string();
                let key = parts
                    .next()
                    .unwrap_or_default()
                    .trim_end_matches(':')
                    .to_string();
                let value = parts.next().unwrap_or_default().to_string();
                DetailEntry { glyph, key, value }
            })
            .collect();

        Self { entries }
    }
}

#[derive(Debug)]
pub struct ActivityMetric {
    pub label: &'static str,
    pub value: String,
}

#[derive(Debug)]
pub struct ActivityViewModel {
    pub metrics: Vec<ActivityMetric>,
}

impl ActivityViewModel {
    pub fn build(state: &AppState) -> Self {
        let activity = state.scan_activity_snapshot();
        let path_value = activity
            .current_path
            .as_deref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "idle".into());

        let metrics = vec![
            ActivityMetric {
                label: "Path",
                value: path_value,
            },
            ActivityMetric {
                label: "Queued dirs",
                value: activity.queued_directories.to_string(),
            },
            ActivityMetric {
                label: "Permission denied",
                value: activity.permission_denied.to_string(),
            },
            ActivityMetric {
                label: "Skipped mounts",
                value: activity.skipped_mounts.to_string(),
            },
            ActivityMetric {
                label: "Skipped symlinks",
                value: activity.skipped_symlinks.to_string(),
            },
            ActivityMetric {
                label: "Files processed",
                value: activity.files_processed.to_string(),
            },
        ];

        Self { metrics }
    }
}

#[derive(Debug)]
pub struct FilesystemViewModel {
    pub table_rows: Vec<Row<'static>>,
    pub filter_prompt: Option<String>,
}

impl FilesystemViewModel {
    pub fn build(state: &mut AppState, area: Rect, theme: Theme) -> Self {
        const OVERLAY_HEIGHT: u16 = 3;
        let tree_rows = collect_tree_rows(
            &state.tree,
            &state.filter_query,
            state.filter_active,
            state.display_options,
        );
        let max_size = tree_rows
            .iter()
            .map(|row| chosen_size(row, state.size_mode, state.display_options))
            .max()
            .unwrap_or(1);

        let visible_height = area.height.saturating_sub(2) as usize;
        if tree_rows.is_empty() {
            state.set_scroll_offset(0);
        }

        let mut offset = state.scroll_offset;
        if !tree_rows.is_empty() && visible_height > 0 {
            let selected_index = tree_rows
                .iter()
                .position(|row| Some(row.id()) == state.selection)
                .unwrap_or(0);
            if selected_index < offset {
                offset = selected_index;
            } else if selected_index >= offset + visible_height {
                offset = selected_index + 1 - visible_height;
            }
            let max_offset = tree_rows.len().saturating_sub(visible_height);
            if offset > max_offset {
                offset = max_offset;
            }
        } else {
            offset = 0;
        }
        state.set_scroll_offset(offset);

        let percent_column_width = (((area.width as usize) * 30) / 100).max(1);
        let size_column_width = (((area.width as usize) * 15) / 100).max(1);
        let table_rows = if visible_height == 0 {
            Vec::new()
        } else {
            tree_rows
                .iter()
                .skip(offset)
                .take(visible_height)
                .map(|row| {
                    build_row(
                        row,
                        state.selection,
                        theme,
                        state.size_mode,
                        max_size,
                        state.display_options,
                        ColumnWidths {
                            percent: percent_column_width,
                            size: size_column_width,
                        },
                    )
                })
                .collect()
        };

        let filter_prompt =
            if state.filter_prompt_active && area.height > OVERLAY_HEIGHT && area.width >= 4 {
                Some(if state.filter_query.is_empty() {
                    " ".into()
                } else {
                    state.filter_query.clone()
                })
            } else {
                None
            };

        Self {
            table_rows,
            filter_prompt,
        }
    }
}
