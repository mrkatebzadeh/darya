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

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub enum ScanTrigger {
    /// Start or restart the configured scan.
    Start,
    /// Pause the running scanner (if any) without canceling.
    Pause,
    /// Resume a paused scanner.
    Resume,
    /// Stop the running scanner and abandon the current run.
    Stop,
    /// Cancel any running scan and signal termination (used during shutdown).
    Cancel,
}

pub type ScanTriggerSender = UnboundedSender<ScanTrigger>;
pub type ScanTriggerReceiver = UnboundedReceiver<ScanTrigger>;
pub type ScanEventSender = UnboundedSender<crate::scan::scanner::ScanEvent>;
pub type ScanEventReceiver = UnboundedReceiver<crate::scan::scanner::ScanEvent>;
