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
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
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

const SCAN_BATCH_SIZE: usize = 512;

/// Indicates the events emitted by the background scanner.
#[derive(Debug)]
pub enum ScanEvent {
    Node(ScanNode),
    Progress(ScanProgress),
    Activity(ScanActivity),
    Error(ScanError),
    Completed,
    Batch(ScanBatch),
}

/// Aggregated payload emitted at batch granularity.
#[derive(Debug)]
pub struct ScanBatch {
    pub nodes: Vec<ScanNode>,
    pub progress: Option<ScanProgress>,
    pub activity: Option<ScanActivity>,
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

/// Snapshot of the scanner activity for UI display.
#[derive(Default, Debug, Clone)]
pub struct ScanActivity {
    pub current_path: Option<PathBuf>,
    pub queued_directories: u64,
    pub permission_denied: u64,
    pub skipped_mounts: u64,
    pub skipped_symlinks: u64,
    pub files_processed: u64,
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
    pub collect_metadata: bool,
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

    let custom_same_fs = options.same_file_system && cfg!(unix);
    #[cfg(unix)]
    let root_dev = if custom_same_fs {
        root.metadata().ok().map(|meta| meta.dev())
    } else {
        None
    };
    #[cfg(not(unix))]
    let root_dev = None;

    let walker = WalkDir::new(&root).follow_links(options.follow_symlinks);
    let walker = if options.same_file_system && !custom_same_fs {
        walker.same_file_system(true)
    } else {
        walker
    };

    let mut walker_iter = walker.into_iter();
    let mut activity = ScanActivity::default();
    let mut dir_stack: Vec<usize> = Vec::new();
    let mut node_batch = Vec::with_capacity(SCAN_BATCH_SIZE);

    while let Some(entry_result) = walker_iter.next() {
        if cancel.load(Ordering::Relaxed) {
            let _ = tx.send(ScanEvent::Completed);
            return;
        }

        match entry_result {
            Ok(entry) => {
                activity.current_path = Some(entry.path().to_path_buf());
                scanned += 1;
                let depth = entry.depth();
                while let Some(&last_depth) = dir_stack.last() {
                    if last_depth >= depth {
                        dir_stack.pop();
                    } else {
                        break;
                    }
                }
                if entry.file_type().is_dir() {
                    dir_stack.push(depth);
                }
                activity.queued_directories = dir_stack.len().saturating_sub(1) as u64;

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

                match entry.metadata() {
                    Ok(metadata) => {
                        if custom_same_fs
                            && let Some(root_dev) = root_dev
                            && let Some(entry_dev) = device_id(&metadata)
                            && entry_dev != root_dev
                        {
                            activity.skipped_mounts += 1;
                            if entry.file_type().is_dir() {
                                walker_iter.skip_current_dir();
                            }
                            let _ = tx.send(ScanEvent::Activity(activity.clone()));
                            continue;
                        }

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

                        let modified = if options.collect_metadata {
                            metadata.modified().ok()
                        } else {
                            None
                        };

                        #[cfg(unix)]
                        let (permissions, uid, gid) = if options.collect_metadata {
                            use std::os::unix::fs::{MetadataExt, PermissionsExt};
                            (
                                Some(metadata.permissions().mode()),
                                Some(metadata.uid()),
                                Some(metadata.gid()),
                            )
                        } else {
                            (None, None, None)
                        };
                        #[cfg(not(unix))]
                        let (permissions, uid, gid) = (None, None, None);

                        let kind = classify(&entry);
                        if kind == NodeType::File {
                            activity.files_processed = activity.files_processed.saturating_add(1);
                        }
                        if kind == NodeType::Symlink && !options.follow_symlinks {
                            activity.skipped_symlinks = activity.skipped_symlinks.saturating_add(1);
                        }

                        node_batch.push(ScanNode {
                            path: entry.path().to_path_buf(),
                            kind,
                            size,
                            disk_size,
                            modified,
                            permissions,
                            uid,
                            gid,
                        });
                        if node_batch.len() >= SCAN_BATCH_SIZE {
                            flush_batch(&tx, &mut node_batch, &activity, scanned, errors);
                        }
                    }
                    Err(err) => {
                        if err
                            .io_error()
                            .map(|inner| inner.kind() == std::io::ErrorKind::PermissionDenied)
                            .unwrap_or(false)
                        {
                            activity.permission_denied =
                                activity.permission_denied.saturating_add(1);
                        }
                        errors += 1;
                        flush_batch(&tx, &mut node_batch, &activity, scanned, errors);
                        let _ = tx.send(ScanEvent::Error(ScanError {
                            path: entry.path().to_path_buf(),
                            source: err,
                        }));
                    }
                }
            }
            Err(err) => {
                activity.current_path =
                    err.path().map(PathBuf::from).or_else(|| Some(root.clone()));
                errors += 1;
                flush_batch(&tx, &mut node_batch, &activity, scanned, errors);
                let _ = tx.send(ScanEvent::Error(ScanError {
                    path: activity
                        .current_path
                        .clone()
                        .unwrap_or_else(|| root.clone()),
                    source: err,
                }));
            }
        }
    }

    flush_batch(&tx, &mut node_batch, &activity, scanned, errors);
    activity.current_path = None;
    let _ = tx.send(ScanEvent::Activity(activity.clone()));
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

fn flush_batch(
    tx: &UnboundedSender<ScanEvent>,
    node_batch: &mut Vec<ScanNode>,
    activity: &ScanActivity,
    scanned: u64,
    errors: u64,
) {
    if node_batch.is_empty() {
        return;
    }
    let nodes = std::mem::take(node_batch);
    node_batch.reserve(SCAN_BATCH_SIZE);
    let batch = ScanBatch {
        nodes,
        progress: Some(ScanProgress { scanned, errors }),
        activity: Some(activity.clone()),
    };
    let _ = tx.send(ScanEvent::Batch(batch));
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

#[cfg(unix)]
fn device_id(metadata: &std::fs::Metadata) -> Option<u64> {
    Some(metadata.dev())
}

#[cfg(not(unix))]
fn device_id(_metadata: &std::fs::Metadata) -> Option<u64> {
    None
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
            collect_metadata: false,
        };
        let (_handle, mut rx) = start_scan(base.clone(), options, Vec::new());
        let mut nodes = 0;
        while let Some(event) = rx.recv().await {
            match event {
                ScanEvent::Batch(batch) => nodes += batch.nodes.len(),
                ScanEvent::Completed => break,
                _ => {}
            }
        }

        assert!(nodes >= 1);
        let _ = fs::remove_dir_all(base);
    }
}
