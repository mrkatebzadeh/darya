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
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreemapNode {
    pub node_id: usize,
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub is_aggregated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreemapTile {
    pub node: TreemapNode,
    pub rect: Rect,
    pub depth: usize,
    pub shade_variant: bool,
}

#[derive(Debug, Clone)]
pub struct TreemapLayout {
    pub tiles: Vec<TreemapTile>,
    pub node_rects: HashMap<usize, Rect>,
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

    let max_nodes = max_nodes.max(1);
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
    let mut row_variant = false;
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
            remaining = layout_row(
                row.as_slice(),
                remaining,
                &mut output,
                None,
                false,
                0,
                &mut row_variant,
            );
            row.clear();
            if remaining.width == 0 || remaining.height == 0 {
                break;
            }
        }
    }

    if !row.is_empty() && remaining.width > 0 && remaining.height > 0 {
        layout_row(
            row.as_slice(),
            remaining,
            &mut output,
            None,
            true,
            0,
            &mut row_variant,
        );
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

    filtered.sort_unstable_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
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
    mut node_rects: Option<&mut HashMap<usize, Rect>>,
    is_last_row: bool,
    depth: usize,
    variant: &mut bool,
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

        let mut current_variant = *variant;
        for (idx, item) in row.iter().enumerate() {
            if remaining_width == 0 {
                break;
            }
            let remaining_items = (row.len() - idx) as u16;
            let width = if idx + 1 == row.len() {
                remaining_width
            } else {
                let max_allowed = remaining_width.saturating_sub(remaining_items - 1);
                if max_allowed == 0 {
                    0
                } else {
                    let estimated = ((f64::from(item.area) / f64::from(row_height.max(1))).round()
                        as u16)
                        .max(1);
                    estimated.clamp(1, max_allowed)
                }
            };

            let tile = TreemapTile {
                node: item.node.clone(),
                rect: Rect::new(x, area.y, width, row_height),
                depth,
                shade_variant: current_variant,
            };
            insert_node_rect(node_rects.as_deref_mut(), &tile);
            output.push(tile);

            x = x.saturating_add(width);
            remaining_width = remaining_width.saturating_sub(width);
            current_variant = !current_variant;
        }

        *variant = !*variant;
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

        let mut current_variant = *variant;
        for (idx, item) in row.iter().enumerate() {
            if remaining_height == 0 {
                break;
            }
            let remaining_items = (row.len() - idx) as u16;
            let height = if idx + 1 == row.len() {
                remaining_height
            } else {
                let max_allowed = remaining_height.saturating_sub(remaining_items - 1);
                if max_allowed == 0 {
                    0
                } else {
                    let estimated = ((f64::from(item.area) / f64::from(row_width.max(1))).round()
                        as u16)
                        .max(1);
                    estimated.clamp(1, max_allowed)
                }
            };

            let tile = TreemapTile {
                node: item.node.clone(),
                rect: Rect::new(area.x, y, row_width, height),
                depth,
                shade_variant: current_variant,
            };
            insert_node_rect(node_rects.as_deref_mut(), &tile);
            output.push(tile);

            y = y.saturating_add(height);
            remaining_height = remaining_height.saturating_sub(height);
            current_variant = !current_variant;
        }

        *variant = !*variant;
        Rect::new(
            area.x.saturating_add(row_width),
            area.y,
            area.width.saturating_sub(row_width),
            area.height,
        )
    }
}

pub fn contextual_treemap_layout<F>(
    root_nodes: &[TreemapNode],
    bounds: Rect,
    selection_path: &[usize],
    max_nodes_per_level: usize,
    child_provider: F,
) -> TreemapLayout
where
    F: FnMut(usize, usize) -> Vec<TreemapNode>,
{
    let max_nodes = max_nodes_per_level.max(2);
    let mut builder = ContextualLayoutBuilder::new(max_nodes, child_provider);
    builder.layout_nodes(root_nodes, bounds, 0, 0);
    builder.layout_selection_path(selection_path);
    builder.finish()
}

struct ContextualLayoutBuilder<F>
where
    F: FnMut(usize, usize) -> Vec<TreemapNode>,
{
    tiles: Vec<TreemapTile>,
    node_rects: HashMap<usize, Rect>,
    max_nodes: usize,
    child_provider: F,
}

impl<F> ContextualLayoutBuilder<F>
where
    F: FnMut(usize, usize) -> Vec<TreemapNode>,
{
    fn new(max_nodes: usize, child_provider: F) -> Self {
        Self {
            tiles: Vec::new(),
            node_rects: HashMap::new(),
            max_nodes,
            child_provider,
        }
    }

    fn layout_nodes(&mut self, nodes: &[TreemapNode], area: Rect, depth: usize, parent_id: usize) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let prepared = prepare_nodes(nodes, self.max_nodes, parent_id);
        if prepared.is_empty() {
            return;
        }

        let mut weighted = normalize_areas(&prepared, area, prepared.len().max(1))
            .into_iter()
            .map(|(node, area)| WeightedNode { node, area })
            .collect::<Vec<_>>();
        weighted.retain(|w| w.area > 0);
        if weighted.is_empty() {
            return;
        }

        let mut remaining = area;
        let mut row: Vec<WeightedNode> = Vec::new();
        let mut row_variant = false;

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
                remaining = layout_row(
                    row.as_slice(),
                    remaining,
                    &mut self.tiles,
                    Some(&mut self.node_rects),
                    false,
                    depth,
                    &mut row_variant,
                );
                row.clear();
                if remaining.width == 0 || remaining.height == 0 {
                    break;
                }
            }
        }

        if !row.is_empty() && remaining.width > 0 && remaining.height > 0 {
            layout_row(
                row.as_slice(),
                remaining,
                &mut self.tiles,
                Some(&mut self.node_rects),
                true,
                depth,
                &mut row_variant,
            );
        }
    }

    fn layout_selection_path(&mut self, path: &[usize]) {
        let mut depth = 1;
        for &node_id in path {
            let parent_rect = match self.node_rects.get(&node_id) {
                Some(rect) => *rect,
                None => break,
            };
            let children = (self.child_provider)(node_id, self.max_nodes);
            self.layout_nodes(&children, parent_rect, depth, node_id);
            depth = depth.saturating_add(1);
        }
    }

    fn finish(self) -> TreemapLayout {
        TreemapLayout {
            tiles: self.tiles,
            node_rects: self.node_rects,
        }
    }
}

fn prepare_nodes(nodes: &[TreemapNode], max_nodes: usize, parent_id: usize) -> Vec<TreemapNode> {
    let mut filtered: Vec<TreemapNode> = nodes.iter().filter(|n| n.size > 0).cloned().collect();

    if filtered.is_empty() {
        return filtered;
    }

    let max_nodes = max_nodes.max(2);
    filtered.sort_unstable_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.node_id.cmp(&b.node_id))
    });

    if filtered.len() <= max_nodes {
        return filtered;
    }

    let other_nodes = filtered.split_off(max_nodes - 1);
    let other_size: u64 = other_nodes.iter().map(|n| n.size).sum();
    filtered.truncate(max_nodes - 1);
    filtered.push(TreemapNode {
        node_id: synthetic_other_id(parent_id),
        name: "other".to_string(),
        size: other_size,
        is_directory: false,
        is_aggregated: true,
    });

    filtered
}

fn synthetic_other_id(parent_id: usize) -> usize {
    usize::MAX.saturating_sub(parent_id)
}

fn insert_node_rect(map: Option<&mut HashMap<usize, Rect>>, tile: &TreemapTile) {
    if !tile.node.is_aggregated {
        if let Some(rect_map) = map {
            rect_map.insert(tile.node.node_id, tile.rect);
        }
    }
}
