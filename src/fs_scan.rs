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

use crate::tree::NodeType;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::SystemTime,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use walkdir::{DirEntry, Error as WalkError, WalkDir};

/// Indicates the events emitted by the background scanner.
#[derive(Debug)]
pub enum ScanEvent {
    Node(ScanNode),
    Progress(ScanProgress),
    Error(ScanError),
    Completed,
}

/// Provides details about a single filesystem entry discovered by the scanner.
#[derive(Debug)]
pub struct ScanNode {
    pub path: PathBuf,
    pub kind: NodeType,
    pub size: u64,
    pub disk_size: u64,
    pub modified: Option<SystemTime>,
    pub permissions: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

/// Progress metrics emitted regularly while scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanProgress {
    pub scanned: u64,
    pub errors: u64,
}

/// Errors emitted during scanning.
#[derive(Debug)]
pub struct ScanError {
    pub path: PathBuf,
    pub source: WalkError,
}

#[derive(Debug, Clone, Copy)]
pub struct ScanOptions {
    pub follow_symlinks: bool,
    pub count_hard_links_once: bool,
    pub same_file_system: bool,
    pub skip_caches: bool,
    pub skip_kernfs: bool,
}

/// Control handle for the background scanner.
#[derive(Clone, Debug)]
pub struct ScannerHandle {
    cancel: Arc<AtomicBool>,
}

impl ScannerHandle {
    pub(crate) fn new(cancel: Arc<AtomicBool>) -> Self {
        Self { cancel }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }
}

/// Start scanning `root` on a background worker and return a receiver for scan events.
pub fn start_scan(
    root: PathBuf,
    options: ScanOptions,
    exclude_patterns: Vec<String>,
) -> (ScannerHandle, UnboundedReceiver<ScanEvent>) {
    let (tx, rx) = unbounded_channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel.clone();
    tokio::task::spawn_blocking(move || {
        let excludes = build_excludes(&exclude_patterns);
        run_scan(root, options, excludes, tx, cancel_clone)
    });
    (ScannerHandle::new(cancel), rx)
}

pub fn dummy_scanner() -> (ScannerHandle, UnboundedReceiver<ScanEvent>) {
    let (tx, rx) = unbounded_channel();
    drop(tx);
    let cancel = Arc::new(AtomicBool::new(true));
    (ScannerHandle::new(cancel), rx)
}

fn run_scan(
    root: PathBuf,
    options: ScanOptions,
    excludes: Option<GlobSet>,
    tx: UnboundedSender<ScanEvent>,
    cancel: Arc<AtomicBool>,
) {
    let mut scanned = 0;
    let mut errors = 0;
    let mut seen_links: HashSet<(u64, u64)> = HashSet::new();

    let walker = WalkDir::new(&root).follow_links(options.follow_symlinks);
    let walker = if options.same_file_system {
        walker.same_file_system(true)
    } else {
        walker
    };

    for entry in walker.into_iter() {
        if cancel.load(Ordering::Relaxed) {
            let _ = tx.send(ScanEvent::Completed);
            return;
        }

        match entry {
            Ok(entry) => {
                if excludes
                    .as_ref()
                    .is_some_and(|set| set.is_match(entry.path()))
                {
                    continue;
                }
                if options.skip_caches && is_cache_entry(&entry) {
                    continue;
                }
                if options.skip_kernfs && is_kernfs_entry(&entry) {
                    continue;
                }
                scanned += 1;
                match entry.metadata() {
                    Ok(metadata) => {
                        let mut size = metadata.len();
                        let mut disk_size = disk_usage_bytes(&metadata);
                        if options.count_hard_links_once
                            && entry.file_type().is_file()
                            && let Some(key) = hard_link_key(&metadata)
                            && !seen_links.insert(key)
                        {
                            size = 0;
                            disk_size = 0;
                        }

                        #[cfg(unix)]
                        let (permissions, uid, gid) = {
                            use std::os::unix::fs::{MetadataExt, PermissionsExt};
                            (
                                Some(metadata.permissions().mode()),
                                Some(metadata.uid()),
                                Some(metadata.gid()),
                            )
                        };
                        #[cfg(not(unix))]
                        let (permissions, uid, gid) = (None, None, None);

                        let node = ScanNode {
                            path: entry.path().to_path_buf(),
                            kind: classify(&entry),
                            size,
                            disk_size,
                            modified: metadata.modified().ok(),
                            permissions,
                            uid,
                            gid,
                        };
                        let _ = tx.send(ScanEvent::Node(node));
                    }
                    Err(err) => {
                        errors += 1;
                        let _ = tx.send(ScanEvent::Error(ScanError {
                            path: entry.path().to_path_buf(),
                            source: err,
                        }));
                    }
                }
                let _ = tx.send(ScanEvent::Progress(ScanProgress { scanned, errors }));
            }
            Err(err) => {
                errors += 1;
                let _ = tx.send(ScanEvent::Error(ScanError {
                    path: err
                        .path()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| root.clone()),
                    source: err,
                }));
                let _ = tx.send(ScanEvent::Progress(ScanProgress { scanned, errors }));
            }
        }
    }

    let _ = tx.send(ScanEvent::Completed);
}

fn build_excludes(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().ok()
}

#[cfg(unix)]
fn hard_link_key(metadata: &std::fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn hard_link_key(_metadata: &std::fs::Metadata) -> Option<(u64, u64)> {
    None
}

#[cfg(unix)]
fn disk_usage_bytes(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
fn disk_usage_bytes(metadata: &std::fs::Metadata) -> u64 {
    metadata.len()
}

fn is_cache_entry(entry: &DirEntry) -> bool {
    entry
        .path()
        .components()
        .any(|component| eq_component(component.as_os_str(), "cache"))
}

fn is_kernfs_entry(entry: &DirEntry) -> bool {
    entry
        .path()
        .components()
        .any(|component| eq_component(component.as_os_str(), "kernfs"))
}

fn eq_component(component: &OsStr, pattern: &str) -> bool {
    component
        .to_str()
        .map(|value| value.eq_ignore_ascii_case(pattern))
        .unwrap_or(false)
}

fn classify(entry: &walkdir::DirEntry) -> NodeType {
    let file_type = entry.file_type();
    if file_type.is_dir() {
        NodeType::Directory
    } else if file_type.is_symlink() {
        NodeType::Symlink
    } else {
        NodeType::File
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
    };

    fn create_tmp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "{name}-{ts}",
            ts = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            name = name
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    fn write_file(path: &Path, size: usize) {
        let mut file = File::create(path).unwrap();
        file.write_all(&vec![0u8; size]).unwrap();
    }

    #[tokio::test]
    async fn scanner_emits_events() {
        let base = create_tmp_dir("dar-scan");
        let file = base.join("file.txt");
        write_file(&file, 4);

        let options = ScanOptions {
            follow_symlinks: false,
            count_hard_links_once: true,
            same_file_system: false,
            skip_caches: false,
            skip_kernfs: false,
        };
        let (_handle, mut rx) = start_scan(base.clone(), options, Vec::new());
        let mut nodes = 0;
        while let Some(event) = rx.recv().await {
            match event {
                ScanEvent::Node(_) => nodes += 1,
                ScanEvent::Completed => break,
                _ => {}
            }
        }

        assert!(nodes >= 1);
        let _ = fs::remove_dir_all(base);
    }
}
