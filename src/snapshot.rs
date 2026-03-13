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

use crate::tree::{FileTree, NodeType};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotEntry {
    path: PathBuf,
    kind: NodeType,
    size: u64,
    disk_size: u64,
    expanded: bool,
}

pub fn export_tree(tree: &FileTree, path: &Path) -> anyhow::Result<()> {
    let entries: Vec<SnapshotEntry> = tree
        .nodes()
        .iter()
        .map(|node| SnapshotEntry {
            path: node.path.clone(),
            kind: node.file_type,
            size: node.size,
            disk_size: node.disk_size,
            expanded: node.expanded,
        })
        .collect();

    let json = serde_json::to_string_pretty(&entries)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn import_tree(path: &Path, default_root: &Path) -> anyhow::Result<FileTree> {
    let contents = fs::read_to_string(path)?;
    let mut entries: Vec<SnapshotEntry> = serde_json::from_str(&contents)?;
    entries.sort_by_key(|entry| entry.path.components().count());

    let root_path = entries
        .first()
        .map(|entry| entry.path.clone())
        .unwrap_or_else(|| default_root.to_path_buf());

    let mut tree = FileTree::new(root_path.clone());
    for entry in entries {
        let id = tree.ensure_node(entry.path.clone(), entry.kind);
        if let Some(node) = tree.node_mut(id) {
            node.size = entry.size;
            node.disk_size = entry.disk_size;
            node.expanded = entry.expanded;
        }
    }

    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::TreeNode;

    #[test]
    fn export_then_import_roundtrip() {
        let root = PathBuf::from("/tmp/dar-snap-root");
        let mut tree = FileTree::new(root.clone());
        let child = tree.add_child(0, TreeNode::new(root.join("child"), NodeType::File));
        if let Some(node) = tree.node_mut(child) {
            node.size = 42;
            node.disk_size = 4096;
        }

        let path = std::env::temp_dir().join("dar-snapshot-test.json");
        export_tree(&tree, &path).unwrap();
        let imported = import_tree(&path, &root).unwrap();

        assert!(imported.nodes().iter().any(|n| n.path.ends_with("child")));
        let _ = fs::remove_file(path);
    }
}
