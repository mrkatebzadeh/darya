# dar
This project is the start of a modern, Rust-based replacement for ncdu that prioritizes responsiveness, configurability, and readable progress.

## Why using this tool
- It begins to solve the pain of slow, hard-to-script disk explorers by combining a rich terminal UI with deliberate scan controls.
- Rather than a drop-in clone, it rethinks ncdu’s workflow: scans only start when you’re ready, progress is tracked explicitly, and every pane is now housed under dedicated modules for clarity.
- Because it embraces a modular architecture, it is easier to extend (for example to add filters, exports, or integrations) than the traditional ncdu codebase.

## How to install it
1. Ensure you have the Rust toolchain installed (https://rustup.rs). Cargo will manage the build/install process.
2. Clone the repository and switch to the `dev` branch if needed:

```
git clone https://github.com/yourorg/dar.git
cd dar
```

3. Build the CLI and run the tests with Cargo:

```
cargo test
cargo build --release
```

4. Install the binary system-wide if desired:

```
```

## How to use it
1. Run the CLI with no arguments to scan the current directory once you press `R`:

```
```

2. While the UI is running, use the keybindings shown in the help modal:
   - `hjkl`: move selection
   - `/`: start filter, `c`: clear
   - `r`: rescan, `R`: start a new scan from root
   - `p/u/x`: pause, resume, stop the scanner
   - `b`, `s`, `E/I`: toggle size mode/sort/export/import

3. Use `--one-file-system`, `--include-caches`, and other CLI flags to control follow-symlinks, caching rules, and thread counts before launching the UI.

4. Import/export snapshots via `-f`, `-o`, or `-O` to share scans across systems or persist results.

## How to config it
- Configuration is loaded from the default TOML (as described in `src/config.rs`) unless you pass `--ignore-config`.
- Overrides in the CLI (such as `--cross-file-system` or `--disk-usage`) take precedence over config values.
- You can adjust display options (`--show-itemcount`, `--no-graph`, etc.) to tailor the table output, and these settings persist only per invocation unless codified in config.

## Any project specific doc
### Architecture
- `src/app/` contains the CLI entrypoint, configuration loader, scan manager, and shared state logic.
- `src/events/` now bifurcates event loop rendering (`event_loop.rs`) from action handlers (`handlers.rs`), allowing cleaner unit tests and targeted reuse.
- `src/ui/` houses the renderer, helpers, layout, themes, and treemap calculations so UI concerns stay grouped.
- `src/scan_control.rs` orchestrates pause/resume/stop/cancel signals that the UI, scan manager, and `fs_scan` module honor.

### Progress and next steps
- Scans wait for user confirmation via `R`, giving you full control over when disk I/O begins.
- Future work includes hooking into configuration presets, exposing metrics, and polishing export/import workflows with a JSON schema.
