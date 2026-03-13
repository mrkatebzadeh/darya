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

use crate::tree::{FileTree, NodeType, TreeNode};
use bincode::config::standard;
use bincode::serde::{decode_from_std_read, encode_into_std_write};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotEntry {
    path: PathBuf,
    kind: NodeType,
    size: u64,
    disk_size: u64,
    expanded: bool,
    permissions: Option<u32>,
    uid: Option<u32>,
    gid: Option<u32>,
    modified_secs: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub enum SnapshotFormat {
    Json,
    Binary,
}

#[derive(Clone, Debug)]
pub enum SnapshotEndpoint {
    StdIo,
    File(PathBuf),
}

impl SnapshotEndpoint {
    fn to_writer(&self) -> anyhow::Result<Box<dyn Write>> {
        match self {
            SnapshotEndpoint::StdIo => Ok(Box::new(io::stdout())),
            SnapshotEndpoint::File(path) => Ok(Box::new(fs::File::create(path)?)),
        }
    }

    fn to_reader(&self) -> anyhow::Result<Box<dyn Read>> {
        match self {
            SnapshotEndpoint::StdIo => Ok(Box::new(io::stdin())),
            SnapshotEndpoint::File(path) => Ok(Box::new(fs::File::open(path)?)),
        }
    }
}

fn write_entries<W: Write>(
    writer: W,
    entries: &[SnapshotEntry],
    format: SnapshotFormat,
) -> anyhow::Result<()> {
    let mut writer = writer;
    match format {
        SnapshotFormat::Json => serde_json::to_writer_pretty(&mut writer, entries)?,
        SnapshotFormat::Binary => {
            let _ = encode_into_std_write(entries, &mut writer, standard())?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn read_entries<R: Read>(reader: R, format: SnapshotFormat) -> anyhow::Result<Vec<SnapshotEntry>> {
    let mut reader = reader;
    let entries = match format {
        SnapshotFormat::Json => serde_json::from_reader(&mut reader)?,
        SnapshotFormat::Binary => {
            decode_from_std_read::<Vec<SnapshotEntry>, _, _>(&mut reader, standard())?
        }
    };
    Ok(entries)
}

fn entry_from_node(node: &TreeNode) -> SnapshotEntry {
    SnapshotEntry {
        path: node.path.clone(),
        kind: node.file_type,
        size: node.size,
        disk_size: node.disk_size,
        expanded: node.expanded,
        permissions: node.permissions,
        uid: node.uid,
        gid: node.gid,
        modified_secs: node
            .modified
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs()),
    }
}

fn build_tree(entries: Vec<SnapshotEntry>, default_root: &Path) -> FileTree {
    let mut entries = entries;
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
            node.permissions = entry.permissions;
            node.uid = entry.uid;
            node.gid = entry.gid;
            node.modified = entry
                .modified_secs
                .and_then(|secs| UNIX_EPOCH.checked_add(Duration::from_secs(secs)));
        }
    }
    tree
}

pub fn export_tree(tree: &FileTree, path: &Path) -> anyhow::Result<()> {
    let entries: Vec<SnapshotEntry> = tree.nodes().iter().map(entry_from_node).collect();
    let writer = fs::File::create(path)?;
    write_entries(writer, &entries, SnapshotFormat::Json)
}

pub fn export_to_destination(
    tree: &FileTree,
    destination: SnapshotEndpoint,
    format: SnapshotFormat,
) -> anyhow::Result<()> {
    let entries: Vec<SnapshotEntry> = tree.nodes().iter().map(entry_from_node).collect();
    let writer = destination.to_writer()?;
    write_entries(writer, &entries, format)
}

pub fn import_from_destination(
    endpoint: SnapshotEndpoint,
    default_root: &Path,
    format: SnapshotFormat,
) -> anyhow::Result<FileTree> {
    let reader = endpoint.to_reader()?;
    let entries = read_entries(reader, format)?;
    Ok(build_tree(entries, default_root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::TreeNode;

    #[test]
    fn export_then_import_roundtrip_json() {
        let root = PathBuf::from("/tmp/dar-snap-root");
        let mut tree = FileTree::new(root.clone());
        let child = tree.add_child(0, TreeNode::new(root.join("child"), NodeType::File));
        if let Some(node) = tree.node_mut(child) {
            node.size = 42;
            node.disk_size = 4096;
        }

        let path = std::env::temp_dir().join("dar-snapshot-test.json");
        export_tree(&tree, &path).unwrap();
        let imported = import_from_destination(
            SnapshotEndpoint::File(path.clone()),
            &root,
            SnapshotFormat::Json,
        )
        .unwrap();

        assert!(imported.nodes().iter().any(|n| n.path.ends_with("child")));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn export_then_import_roundtrip_binary() {
        let root = PathBuf::from("/tmp/dar-snap-root");
        let mut tree = FileTree::new(root.clone());
        let child = tree.add_child(0, TreeNode::new(root.join("binary-child"), NodeType::File));
        if let Some(node) = tree.node_mut(child) {
            node.size = 123;
            node.disk_size = 8192;
        }

        let path = std::env::temp_dir().join("dar-snapshot-test.bin");
        export_to_destination(
            &tree,
            SnapshotEndpoint::File(path.clone()),
            SnapshotFormat::Binary,
        )
        .unwrap();
        let imported = import_from_destination(
            SnapshotEndpoint::File(path.clone()),
            &root,
            SnapshotFormat::Binary,
        )
        .unwrap();

        assert!(
            imported
                .nodes()
                .iter()
                .any(|n| n.path.ends_with("binary-child"))
        );
        let _ = fs::remove_file(path);
    }
}
