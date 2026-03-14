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
pub type ScanEventSender = UnboundedSender<crate::fs_scan::ScanEvent>;
pub type ScanEventReceiver = UnboundedReceiver<crate::fs_scan::ScanEvent>;
