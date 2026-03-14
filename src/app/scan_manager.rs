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

use crate::fs_scan;
use crate::fs_scan::ScanOptions;
use crate::scan_control::{ScanEventSender, ScanTrigger, ScanTriggerReceiver};
use std::path::PathBuf;

pub(crate) async fn scan_manager(
    mut commands: ScanTriggerReceiver,
    event_tx: ScanEventSender,
    root: PathBuf,
    exclude_patterns: Vec<String>,
    scan_options: ScanOptions,
) {
    let mut running_handle: Option<fs_scan::ScannerHandle> = None;
    while let Some(command) = commands.recv().await {
        match command {
            ScanTrigger::Start => {
                if let Some(handle) = running_handle.take() {
                    handle.cancel();
                }
                let root_clone = root.clone();
                let patterns = exclude_patterns.clone();
                let (handle, mut scanner_rx) =
                    fs_scan::start_scan(root_clone, scan_options, patterns);
                let sender = event_tx.clone();
                running_handle = Some(handle);
                tokio::spawn(async move {
                    while let Some(event) = scanner_rx.recv().await {
                        let _ = sender.send(event);
                    }
                });
            }
            ScanTrigger::Stop | ScanTrigger::Cancel => {
                if let Some(handle) = running_handle.take() {
                    handle.cancel();
                }
                if let ScanTrigger::Cancel = command {
                    break;
                }
            }
            ScanTrigger::Pause | ScanTrigger::Resume => {
                // Not supported in current scanner; ignore.
            }
        }
    }
    if let Some(handle) = running_handle {
        handle.cancel();
    }
}
