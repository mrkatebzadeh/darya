<div align="center">
  <h1>darya</h1>
  <p>`darya` is a lightweight disk usage tool built for the terminal. It stays at home on headless servers and laptops alike, giving you a quick way to see what is eating space without requiring a graphical environment.</p>
  <p>The interface is straightforward and easy to navigate, letting you move through directories, scan on demand, and focus on the information that matters while keeping the experience fast and responsive across POSIX systems.</p>
  <p>
    [![Build](https://github.com/mrkatebzadeh/darya/actions/workflows/ci.yaml/badge.svg?branch=main)](https://github.com/mrkatebzadeh/darya/actions/workflows/ci.yaml)
    [![Coverage](https://codecov.io/gh/mrkatebzadeh/darya/branch/main/graph/badge.svg)](https://codecov.io/gh/mrkatebzadeh/darya)
    [![Crates](https://img.shields.io/crates/v/darya.svg)](https://crates.io/crates/darya)
  </p>
</div>

---

<img width="1420" height="841" alt="Image" src="https://github.com/user-attachments/assets/a4ffa94c-aef4-4e30-bd12-2a3bcfc00cba" />

## Installation
### 1) Cargo install
1. Make sure Rust and Cargo are available on your system, then install the latest published build with the command below.
   ```bash
   cargo install darya
   ```

### 2) Download from the releases page
1. Download the archive that matches your platform from the [darya releases page](https://github.com/mrkatebzadeh/darya/releases) using the command below.
   ```bash
   curl -LO https://github.com/mrkatebzadeh/darya/releases/latest/download/darya-<platform>.tar.gz
   ```
2. Extract the archive and move the binary onto your `PATH` (for example, `/usr/local/bin`) by running the commands below.
   ```bash
   tar -xzf darya-<platform>.tar.gz
   sudo mv darya /usr/local/bin/
   ```
3. Optionally verify the release checksum before running the binary with the command below.
   ```bash
   sha256sum darya-<platform>.tar.gz
   ```

### 3) Building from source
1. Clone the repository and reset to the desired release tag (skip if you already have the source) by running the command below.
   ```bash
   git clone https://github.com/mrkatebzadeh/darya && cd darya
   ```
2. Run the test suite and release build commands below.
   ```bash
   cargo test
   cargo build --release
   ```
3. Install the freshly built binary using Cargo with the command below.
   ```bash
   cargo install --path .
   ```

## Quick start
- Launch `darya` in any directory to open the UI, then press `R` to start a scan.
- Navigate with the keybindings shown at the bottom of the UI, toggle filters with `/` and `c`, and switch sorting/size modes with the letters displayed in the help pane.
- Use `-x`/`--one-file-system`, `-y`/`--show-hidden`, or the other CLI flags if you need to control what the scanner visits before you start the UI.
- Export scans with the provided snapshot flags (`-f`, `-o`, `-O`) to share what you have found without rerunning a full scan.

## Keybindings
| Key | Action |
| --- | --- |
| `k` / `↑` | Move the selection up. |
| `j` / `↓` | Move the selection down. |
| `h` / `←` | Collapse the highlighted directory or file group. |
| `l` / `→` | Expand the highlighted directory or file group. |
| `gg` | Jump to the top of the tree. |
| `G` | Jump to the bottom of the tree. |
| `Enter` / `Tab` | Select the current item. |
| `R` | Start a new scan for the current directory. |
| `r` | Rescan the currently selected path. |
| `/` | Begin typing to filter the tree. Press `Enter` to apply or `Esc` to exit filter mode. |
| `c` | Clear the active filter. |
| `s` | Cycle through the available sort modes. |
| `b` | Toggle the size display mode (bytes, percentage, etc.). |
| `E` | Export the latest scan snapshot. |
| `I` | Import a previously exported scan snapshot. |
| `d` | Delete the selected entry from the scan tree. |
| `o` | Open the selected entry in your default viewer. |
| `H` | Toggle showing hidden files and directories. |
| `?` / `Esc` | Open or close the help overlay. |
| `q` | Quit the application. |

## Configuration and customization
- Settings come from `~/.config/darya/config.toml` and provide defaults for sorting, UI tweaks, and scan filters. Pass `--ignore-config` to skip it.
- Overrides on the command line always win, so you can keep a mild default configuration and still tweak behavior at runtime.
