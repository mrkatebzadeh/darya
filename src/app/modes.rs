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

use super::ui_thread::run_ui_thread;
use crate::config::Config;
use crate::events::process_scan_event;
use crate::fs_scan::{self, ScanEvent, ScanOptions, ScanProgress, dummy_scanner};
use crate::snapshot::{self, ExportOptions, SnapshotEndpoint, SnapshotFormat};
use crate::state::{AppState, ScanState};
use crate::theme::Theme;
use anyhow::Result;
use std::path::PathBuf;

pub(crate) struct ExportDestinations {
    pub json: Option<SnapshotEndpoint>,
    pub binary: Option<SnapshotEndpoint>,
}

pub(crate) async fn run_import_mode(
    root: PathBuf,
    sort_mode: crate::config::SortMode,
    endpoint: SnapshotEndpoint,
    theme: Theme,
    extended: bool,
) -> Result<()> {
    let mut state = AppState::new(root.clone(), sort_mode);
    state.set_extended_mode(extended);
    let default_root = state
        .tree
        .node(state.tree.root())
        .map(|node| node.path.clone())
        .unwrap_or(root.clone());
    state.tree = snapshot::import_from_destination(endpoint, &default_root, SnapshotFormat::Json)?;
    state.selection = Some(state.tree.root());
    state.allow_modifications = false;
    let (_scanner_handle, scanner_rx) = dummy_scanner();
    let (scan_trigger_tx, _scan_trigger_rx) =
        tokio::sync::mpsc::unbounded_channel::<crate::scan_control::ScanTrigger>();
    run_ui_thread(state, scanner_rx, scan_trigger_tx, theme).await
}

pub(crate) async fn run_export_mode(
    root: PathBuf,
    exclude_patterns: Vec<String>,
    config: &Config,
    export_destinations: ExportDestinations,
    extended: bool,
    scan_options: ScanOptions,
    export_options: ExportOptions,
) -> Result<()> {
    let (scanner_handle, mut scanner_rx) =
        fs_scan::start_scan(root.clone(), scan_options, exclude_patterns);
    let mut state = AppState::new(root, config.sorting.mode);
    state.set_extended_mode(extended);
    state.set_export_options(export_options);
    state.mark_scan_progress(ScanProgress {
        scanned: 0,
        errors: 0,
    });
    while let Some(event) = scanner_rx.recv().await {
        process_scan_event(&mut state, event);
        if matches!(state.scan_state, ScanState::Completed) {
            break;
        }
    }
    let json_options = ExportOptions {
        format: SnapshotFormat::Json,
        ..export_options
    };
    if let Some(dest) = export_destinations.json {
        snapshot::export_to_destination(&state.tree, dest, json_options)?;
    }
    if let Some(dest) = export_destinations.binary {
        let binary_options = ExportOptions {
            format: SnapshotFormat::Binary,
            compress: false,
            ..export_options
        };
        snapshot::export_to_destination(&state.tree, dest, binary_options)?;
    }
    scanner_handle.cancel();
    Ok(())
}

pub(crate) async fn run_progress_mode(
    root: PathBuf,
    exclude_patterns: Vec<String>,
    scan_options: ScanOptions,
) -> Result<()> {
    let (scanner_handle, mut scanner_rx) =
        fs_scan::start_scan(root.clone(), scan_options, exclude_patterns);
    while let Some(event) = scanner_rx.recv().await {
        match event {
            ScanEvent::Progress(progress) => {
                println!(
                    "progress: scanned {} errors {}",
                    progress.scanned, progress.errors
                );
            }
            ScanEvent::Error(err) => {
                eprintln!("scan error for {}: {}", err.path.display(), err.source);
            }
            ScanEvent::Completed => break,
            _ => {}
        }
    }
    scanner_handle.cancel();
    Ok(())
}

pub(crate) async fn run_summary_mode(
    root: PathBuf,
    exclude_patterns: Vec<String>,
    scan_options: ScanOptions,
) -> Result<()> {
    let (scanner_handle, mut scanner_rx) =
        fs_scan::start_scan(root.clone(), scan_options, exclude_patterns);
    let mut last = ScanProgress {
        scanned: 0,
        errors: 0,
    };
    while let Some(event) = scanner_rx.recv().await {
        match event {
            ScanEvent::Progress(progress) => {
                last = progress;
            }
            ScanEvent::Error(err) => {
                eprintln!("scan error for {}: {}", err.path.display(), err.source);
            }
            ScanEvent::Completed => break,
            _ => {}
        }
    }
    println!("summary: scanned {} errors {}", last.scanned, last.errors);
    scanner_handle.cancel();
    Ok(())
}

pub(crate) fn export_options_from_cli(cli: &crate::cli::CliArgs) -> ExportOptions {
    let base = ExportOptions::default();
    ExportOptions {
        compress: cli.export_compress,
        compress_level: cli
            .export_compress_level
            .unwrap_or(base.compress_level)
            .min(9),
        block_size: cli.export_block_size.unwrap_or(base.block_size),
        ..base
    }
}
