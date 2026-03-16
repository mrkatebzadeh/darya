use dar::app::fs_scan::{ScanOptions, start_scan};
use dar::app::state::{AppState, ScanState};
use dar::config::SortMode;
use dar::events::controller::process_scan_event;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn scan_aggregates_sizes_from_children() {
    let tmp = tempdir().expect("tmpdir");
    let root = tmp.path().to_path_buf();
    let nested = root.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(root.join("file_a"), vec![0u8; 4096]).unwrap();
    fs::write(nested.join("file_b"), vec![0u8; 8192]).unwrap();

    let options = ScanOptions {
        follow_symlinks: false,
        count_hard_links_once: true,
        same_file_system: false,
        skip_caches: false,
        skip_kernfs: false,
        collect_metadata: true,
    };

    let (handle, mut rx) = start_scan(root.clone(), options, Vec::new());
    let mut state = AppState::new(root.clone(), SortMode::SizeDesc);
    state.set_extended_mode(true);

    while let Some(event) = rx.recv().await {
        process_scan_event(&mut state, event);
        if matches!(state.scan_state, ScanState::Completed) {
            break;
        }
    }
    handle.cancel();

    assert!(state.tree.verify_size_invariants());
    let expected = 4096 + 8192;
    assert_eq!(state.tree.node(0).unwrap().size, expected);
}
