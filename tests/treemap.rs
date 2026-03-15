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

use dar::treemap::{TreemapNode, normalize_areas, squarified_treemap};
use ratatui::layout::Rect;

fn sample_nodes() -> Vec<TreemapNode> {
    vec![
        TreemapNode {
            node_id: 1,
            name: "alpha".to_string(),
            size: 400,
            is_directory: true,
            is_aggregated: false,
        },
        TreemapNode {
            node_id: 2,
            name: "beta".to_string(),
            size: 250,
            is_directory: false,
            is_aggregated: false,
        },
        TreemapNode {
            node_id: 3,
            name: "gamma".to_string(),
            size: 200,
            is_directory: true,
            is_aggregated: false,
        },
        TreemapNode {
            node_id: 4,
            name: "delta".to_string(),
            size: 100,
            is_directory: false,
            is_aggregated: false,
        },
        TreemapNode {
            node_id: 5,
            name: "zero".to_string(),
            size: 0,
            is_directory: false,
            is_aggregated: false,
        },
    ]
}

fn area(rect: Rect) -> u32 {
    u32::from(rect.width) * u32::from(rect.height)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    let ax2 = a.x + a.width;
    let ay2 = a.y + a.height;
    let bx2 = b.x + b.width;
    let by2 = b.y + b.height;

    a.x < bx2 && b.x < ax2 && a.y < by2 && b.y < ay2
}

#[test]
fn normalization_preserves_total_panel_area() {
    let bounds = Rect::new(0, 0, 80, 30);
    let normalized = normalize_areas(&sample_nodes(), bounds, 200);
    let total: u32 = normalized.iter().map(|(_, area)| *area).sum();
    assert_eq!(total, area(bounds));
    assert!(normalized.iter().all(|(node, _)| node.size > 0));
}

#[test]
fn rectangles_are_inside_bounds_and_non_overlapping() {
    let bounds = Rect::new(2, 3, 90, 25);
    let tiles = squarified_treemap(&sample_nodes(), bounds, 200);
    assert!(!tiles.is_empty());

    for tile in &tiles {
        assert!(tile.rect.x >= bounds.x);
        assert!(tile.rect.y >= bounds.y);
        assert!(tile.rect.x + tile.rect.width <= bounds.x + bounds.width);
        assert!(tile.rect.y + tile.rect.height <= bounds.y + bounds.height);
    }

    for (idx, left) in tiles.iter().enumerate() {
        for right in tiles.iter().skip(idx + 1) {
            assert!(!overlaps(left.rect, right.rect));
        }
    }
}

#[test]
fn rectangle_areas_cover_panel_area() {
    let bounds = Rect::new(0, 0, 64, 20);
    let tiles = squarified_treemap(&sample_nodes(), bounds, 200);
    let covered: u32 = tiles.iter().map(|tile| area(tile.rect)).sum();
    assert_eq!(covered, area(bounds));
}

#[test]
fn respects_max_node_limit() {
    let bounds = Rect::new(0, 0, 100, 20);
    let tiles = squarified_treemap(&sample_nodes(), bounds, 2);
    assert_eq!(tiles.len(), 2);
}

#[test]
fn small_panel_never_overflows() {
    let mut nodes = Vec::new();
    for i in 0..64 {
        nodes.push(TreemapNode {
            node_id: i,
            name: format!("n{i}"),
            size: 1,
            is_directory: false,
            is_aggregated: false,
        });
    }

    let bounds = Rect::new(0, 0, 8, 4);
    let tiles = squarified_treemap(&nodes, bounds, 200);
    for tile in tiles {
        assert!(tile.rect.x + tile.rect.width <= bounds.x + bounds.width);
        assert!(tile.rect.y + tile.rect.height <= bounds.y + bounds.height);
    }
}
