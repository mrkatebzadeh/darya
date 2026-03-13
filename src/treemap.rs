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

use ratatui::layout::Rect;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreemapNode {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreemapTile {
    pub node: TreemapNode,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
struct WeightedNode {
    node: TreemapNode,
    area: u32,
}

pub fn squarified_treemap(
    nodes: &[TreemapNode],
    bounds: Rect,
    max_nodes: usize,
) -> Vec<TreemapTile> {
    if bounds.width == 0 || bounds.height == 0 || max_nodes == 0 {
        return Vec::new();
    }

    let mut weighted = normalize_areas(nodes, bounds, max_nodes)
        .into_iter()
        .map(|(node, area)| WeightedNode { node, area })
        .collect::<Vec<_>>();
    weighted.retain(|w| w.area > 0);
    if weighted.is_empty() {
        return Vec::new();
    }

    let mut remaining = bounds;
    let mut row: Vec<WeightedNode> = Vec::new();
    let mut output = Vec::with_capacity(weighted.len());

    while let Some(candidate) = weighted.first().cloned() {
        if row.is_empty() {
            row.push(candidate);
            weighted.remove(0);
            continue;
        }

        let side = f64::from(remaining.width.min(remaining.height).max(1));
        let current_worst = worst_ratio(&row, side);

        let mut with_candidate = row.clone();
        with_candidate.push(candidate.clone());
        let next_worst = worst_ratio(&with_candidate, side);

        if next_worst <= current_worst {
            row.push(candidate);
            weighted.remove(0);
        } else {
            remaining = layout_row(row.as_slice(), remaining, &mut output, false);
            row.clear();
            if remaining.width == 0 || remaining.height == 0 {
                break;
            }
        }
    }

    if !row.is_empty() && remaining.width > 0 && remaining.height > 0 {
        layout_row(row.as_slice(), remaining, &mut output, true);
    }

    output
}

pub fn normalize_areas(
    nodes: &[TreemapNode],
    bounds: Rect,
    max_nodes: usize,
) -> Vec<(TreemapNode, u32)> {
    if bounds.width == 0 || bounds.height == 0 || max_nodes == 0 {
        return Vec::new();
    }

    let mut filtered: Vec<TreemapNode> = nodes.iter().filter(|n| n.size > 0).cloned().collect();
    filtered.sort_unstable_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    filtered.truncate(max_nodes);

    let total_cells = u32::from(bounds.width) * u32::from(bounds.height);
    normalized_areas(&filtered, total_cells)
        .into_iter()
        .map(|weighted| (weighted.node, weighted.area))
        .collect()
}

fn normalized_areas(nodes: &[TreemapNode], total_cells: u32) -> Vec<WeightedNode> {
    let total_size: u64 = nodes.iter().map(|n| n.size).sum();
    if total_size == 0 || total_cells == 0 {
        return Vec::new();
    }

    let mut entries: Vec<(TreemapNode, u32, f64)> = nodes
        .iter()
        .cloned()
        .map(|node| {
            let raw = node.size as f64 * f64::from(total_cells) / total_size as f64;
            let base = raw.floor() as u32;
            (node, base, raw - f64::from(base))
        })
        .collect();

    let base_sum: u32 = entries.iter().map(|(_, base, _)| *base).sum();
    let mut remaining = total_cells.saturating_sub(base_sum);

    entries.sort_unstable_by(|a, b| b.2.total_cmp(&a.2));
    for (_, base, _) in entries.iter_mut() {
        if remaining == 0 {
            break;
        }
        *base = base.saturating_add(1);
        remaining -= 1;
    }

    entries
        .into_iter()
        .map(|(node, area, _)| WeightedNode { node, area })
        .collect()
}

fn worst_ratio(row: &[WeightedNode], side: f64) -> f64 {
    if row.is_empty() {
        return f64::INFINITY;
    }

    let sum: f64 = row.iter().map(|n| f64::from(n.area)).sum();
    let min_area = row.iter().map(|n| n.area).min().unwrap_or(1) as f64;
    let max_area = row.iter().map(|n| n.area).max().unwrap_or(1) as f64;

    let side_sq = side * side;
    let sum_sq = sum * sum;

    ((side_sq * max_area) / sum_sq).max(sum_sq / (side_sq * min_area))
}

fn layout_row(
    row: &[WeightedNode],
    area: Rect,
    output: &mut Vec<TreemapTile>,
    is_last_row: bool,
) -> Rect {
    if row.is_empty() || area.width == 0 || area.height == 0 {
        return area;
    }

    let row_area: u32 = row.iter().map(|node| node.area).sum();
    let horizontal = area.width >= area.height;

    if horizontal {
        let mut row_height = if is_last_row {
            area.height
        } else {
            ((row_area as f64 / f64::from(area.width)).round() as u16).clamp(1, area.height)
        };
        if row_height > area.height {
            row_height = area.height;
        }

        let mut x = area.x;
        let mut remaining_width = area.width;

        for (idx, item) in row.iter().enumerate() {
            let remaining_items = (row.len() - idx) as u16;
            let width = if idx + 1 == row.len() {
                remaining_width
            } else {
                let estimated =
                    ((f64::from(item.area) / f64::from(row_height.max(1))).round() as u16).max(1);
                estimated.clamp(
                    1,
                    remaining_width.saturating_sub(remaining_items - 1).max(1),
                )
            };

            output.push(TreemapTile {
                node: item.node.clone(),
                rect: Rect::new(x, area.y, width, row_height),
            });

            x = x.saturating_add(width);
            remaining_width = remaining_width.saturating_sub(width);
        }

        Rect::new(
            area.x,
            area.y.saturating_add(row_height),
            area.width,
            area.height.saturating_sub(row_height),
        )
    } else {
        let mut row_width = if is_last_row {
            area.width
        } else {
            ((row_area as f64 / f64::from(area.height)).round() as u16).clamp(1, area.width)
        };
        if row_width > area.width {
            row_width = area.width;
        }

        let mut y = area.y;
        let mut remaining_height = area.height;

        for (idx, item) in row.iter().enumerate() {
            let remaining_items = (row.len() - idx) as u16;
            let height = if idx + 1 == row.len() {
                remaining_height
            } else {
                let estimated =
                    ((f64::from(item.area) / f64::from(row_width.max(1))).round() as u16).max(1);
                estimated.clamp(
                    1,
                    remaining_height.saturating_sub(remaining_items - 1).max(1),
                )
            };

            output.push(TreemapTile {
                node: item.node.clone(),
                rect: Rect::new(area.x, y, row_width, height),
            });

            y = y.saturating_add(height);
            remaining_height = remaining_height.saturating_sub(height);
        }

        Rect::new(
            area.x.saturating_add(row_width),
            area.y,
            area.width.saturating_sub(row_width),
            area.height,
        )
    }
}
