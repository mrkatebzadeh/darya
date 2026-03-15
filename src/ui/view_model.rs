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
use crate::ui::helpers::detail_panel_lines;

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
