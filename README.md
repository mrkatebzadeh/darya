# dar
[![Build](https://github.com/mrkatebzadeh/dar/actions/workflows/ci.yaml/badge.svg?branch=main)](https://github.com/mrkatebzadeh/dar/actions/workflows/ci.yaml)
[![Coverage](https://codecov.io/gh/mrkatebzadeh/dar/branch/main/graph/badge.svg)](https://codecov.io/gh/mrkatebzadeh/dar)

`dar` is a terminal-first disk audit runner that keeps things responsive and readable while you explore storage. It’s in the same spirit as `du`, but TUI-based so you can scroll through directories without losing track of what’s happening. It lets you decide when scanning happens, keeps the panels focused, and surfaces the details you need without overwhelming you with noise.

![dar](https://github.com/user-attachments/assets/548e252f-76e7-4ea5-acf6-e1ca764bbe37)

## Installation
1. Visit the [dar releases page](https://github.com/yourorg/dar/releases) and download the latest archive for your platform.
2. Extract the archive and place the `dar` binary somewhere on your `PATH` (for example, `/usr/local/bin`).
3. Optionally, verify the download by checking the accompanying checksum before you run it.

If you're building from source, run `cargo test` and `cargo build --release` from the repository root, then install the resulting binary with `cargo install --path .`.

## Quick start
- Launch `dar` in any directory to open the UI, then press `R` to start a scan.
- Navigate with the keybindings shown at the bottom of the UI, toggle filters with `/` and `c`, and switch sorting/size modes with the letters shown in the help pane.
- Use `-x`/`--one-file-system`, `-y`/`--show-hidden`, or the other CLI flags if you need to control what the scanner visits before you start the UI.
- Export scans with the provided snapshot flags (`-f`, `-o`, `-O`) to share what you’ve found without rerunning a full scan.

## Configuration and customization
- Settings come from `~/.config/dar/config.toml` and provide defaults for sorting, UI tweaks, and scan filters. Pass `--ignore-config` to skip it.
- Overrides on the command line always win, so you can keep a mild default configuration and still tweak behavior at runtime.
