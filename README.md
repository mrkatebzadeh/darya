<div align="center">
  <h1>DARYA</h1>
  <h4>A Fast and Keyboard-Driven Disk Usage Explorer</h4>
  <h5> Built with Rust, Ratatui, and Crossterm</h5>
  <p>
    <a href="https://github.com/mrkatebzadeh/darya/actions/workflows/ci.yaml">
      <img src="https://github.com/mrkatebzadeh/darya/actions/workflows/ci.yaml/badge.svg?branch=main" alt="Build badge" />
    </a>
    <a href="https://codecov.io/gh/mrkatebzadeh/darya">
      <img src="https://codecov.io/gh/mrkatebzadeh/darya/branch/main/graph/badge.svg" alt="Coverage badge" />
    </a>
    <a href="https://crates.io/crates/darya">
      <img src="https://img.shields.io/crates/v/darya.svg" alt="Crates badge" />
    </a>
  </p>
</div>

---

`darya` is a lightweight disk usage tool built for the terminal. It’s just as at home on a headless server as it is on your laptop, giving you a quick way to see what’s taking up space without needing a graphical environment.

The interface is straightforward and easy to navigate, letting you move through directories, run scans when you want, and focus on the information that actually matters. It’s designed to stay fast and responsive, keeping things simple while working reliably across POSIX-like systems.

Darya now also runs on Windows (MSVC), so the same keyboard-driven UI is available inside Windows terminals just like on Linux and macOS.

<img src="https://github.com/user-attachments/assets/9977c103-7b4b-4734-8ff4-4781b65be9c9" />

## Installation
### 1) Install script (fastest)
1. Run the install script below to fetch the latest release for your platform.
   ```bash
   curl -fsSL https://mr.katebzadeh.xyz/tools/darya/install | bash
   ```

### 2) Cargo install
1. Make sure Rust and Cargo are available on your system, then install the latest published build with the command below.
   ```bash
   cargo install darya
   ```

### 3) Download from the releases page
1. Download the archive that matches your platform from the [darya releases page](https://github.com/mrkatebzadeh/darya/releases) using the command below.
   ```bash
   curl -LO https://github.com/mrkatebzadeh/darya/releases/latest/download/darya-<platform>.tar.gz
   ```
2. Extract the archive and move the binary onto your `PATH` (for example, `/usr/local/bin`) by running the commands below.
   ```bash
   tar -xzf darya-<platform>.tar.gz
   sudo mv darya /usr/local/bin/
   ```

### 4) Building from source
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

### Windows support
- Install the Visual Studio Build Tools (or the full Visual Studio Desktop C++ workload) to pull in the MSVC linker and headers required by the Windows toolchain.
- Add the MSVC Rust target with:
  ```bash
  rustup target add x86_64-pc-windows-msvc
  ```
- From a Developer Command Prompt or an administrative PowerShell session, run:
  ```bash
  cargo install darya
  ```
- Run `cargo test` followed by `cargo build --release` on Windows to validate the build before tagging a release; the info panel in the UI surfaces Windows-specific attributes when the app is running on that platform.

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
| `t` | Toggle the treemap panel. |
| `q` | Quit the application. |

## Configuration and customization
- Settings come from a `config.toml` in your system config directory. On Linux this is `~/.config/darya/config.toml`, while on macOS it lives at `~/Library/Application Support/darya/config.toml`. Pass `--ignore-config` to skip loading it.
- Overrides on the command line always win, so you can keep a mild default configuration and still tweak behavior at runtime.


| Section | Key | Type | Description |
| --- | --- | --- | --- |
| `sorting` | `mode` | string | Default sort mode (`size_desc`, `size_asc`, `name`, `modified_time`). |
| `scan` | `exclude_patterns` | array of strings | Glob patterns to skip during scans. |
| `scan` | `follow_symlinks` | bool | Follow symlink targets during scans. |
| `scan` | `one_file_system` | bool | Stay on the same filesystem as the root path. |
| `scan` | `exclude_caches` | bool | Skip cache directories when scanning. |
| `scan` | `exclude_kernfs` | bool | Skip kernel filesystem paths when scanning. |
| `scan` | `count_hard_links_once` | bool | Count hard-linked files only once. |
| `scan` | `thread_count` | integer | Worker threads for scanning (0 uses a single thread). |
| `theme` | `background` | string | Base background color name. |
| `theme` | `foreground` | string | Base foreground color name. |
| `theme` | `directory` | string | Color for directory entries. |
| `theme` | `file` | string | Color for file entries. |
| `theme` | `symlink` | string | Color for symlink entries. |
| `theme` | `other` | string | Color for other entry types. |
| `theme` | `tile_palette` | array of strings | Treemap tile palette colors. |
