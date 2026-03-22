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
// GNU General Public License for details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{display::DisplayOptions, snapshot::SnapshotEndpoint};
use clap::Parser;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Ncdu-inspired disk usage explorer.
///
/// See https://dev.yorhel.nl/ncdu/man for the reference ncdu manual.
#[derive(Debug, Parser)]
#[command(name = "darya", version, about, long_about = None)]
pub struct DaryaCli {
    /// Optional starting directory (default: current working directory)
    #[arg(value_name = "PATH")]
    pub root: Option<PathBuf>,

    /// Import snapshot from FILE (use '-' for stdin, JSON only)
    #[arg(short = 'f', long, value_name = "FILE")]
    pub import_snapshot: Option<String>,

    /// Export scan tree to FILE in JSON format
    #[arg(short = 'o', value_name = "FILE")]
    pub export_json: Option<String>,

    /// Export scan tree to FILE in binary format
    #[arg(short = 'O', value_name = "FILE")]
    pub export_binary: Option<String>,

    /// Enable extended metadata mode for owner/permissions/mtime
    #[arg(short = 'e', long)]
    pub extended: bool,

    /// Disable extended metadata mode
    #[arg(long)]
    pub no_extended: bool,

    /// Stay on the starting filesystem
    #[arg(short = 'x', long)]
    pub one_file_system: bool,

    /// Allow crossing filesystem boundaries
    #[arg(long)]
    pub cross_file_system: bool,

    /// Follow symbolic links while scanning
    #[arg(short = 'L', long)]
    pub follow_symlinks: bool,

    /// Do not follow symlinks
    #[arg(long)]
    pub no_follow_symlinks: bool,

    /// Explicitly include cache directories
    #[arg(long)]
    pub include_caches: bool,

    /// Skip directories named cache
    #[arg(long)]
    pub exclude_caches: bool,

    /// Include kernfs-mounted directories
    #[arg(long)]
    pub include_kernfs: bool,

    /// Skip kernfs namespaces
    #[arg(long)]
    pub exclude_kernfs: bool,

    /// Limit the runtime worker threads to N
    #[arg(short = 't', long, value_name = "N")]
    pub thread_count: Option<usize>,

    /// Compress the exported JSON with gzip
    #[arg(short = 'c', long)]
    pub compress: bool,

    /// Set gzip compression level (1-9)
    #[arg(long, value_name = "N")]
    pub compress_level: Option<u32>,

    /// Set buffered block size for exports
    #[arg(long, value_name = "BYTES")]
    pub export_block_size: Option<usize>,

    /// Force UI mode (default)
    #[arg(long = "0")]
    pub force_tui: bool,

    /// Print a textual progress report instead of the UI
    #[arg(long = "1")]
    pub force_progress: bool,

    /// Print a simple summary after scanning
    #[arg(long = "2")]
    pub force_summary: bool,

    /// Do not load configuration files
    #[arg(long)]
    pub ignore_config: bool,

    /// Exclude files/directories by glob pattern (repeatable)
    #[arg(long, value_name = "PATTERN")]
    pub exclude: Vec<String>,

    /// Read exclude patterns from FILE (one pattern per line, # for comments)
    #[arg(short = 'X', long = "exclude-from", value_name = "FILE")]
    pub exclude_from: Vec<PathBuf>,

    // Display options
    /// Use SI units (1k = 1000) instead of binary (1k = 1024)
    #[arg(long)]
    pub si: bool,

    /// Show disk usage instead of apparent size
    #[arg(long)]
    pub disk_usage: bool,

    /// Show apparent size instead of disk usage
    #[arg(long)]
    pub apparent_size: bool,

    #[arg(long)]
    pub show_hidden: bool,

    #[arg(long)]
    pub hide_hidden: bool,

    #[arg(long)]
    pub show_itemcount: bool,

    #[arg(long)]
    pub hide_itemcount: bool,

    #[arg(long)]
    pub show_mtime: bool,

    #[arg(long)]
    pub hide_mtime: bool,

    #[arg(long)]
    pub show_percent: bool,

    #[arg(long)]
    pub hide_percent: bool,

    /// Disable the size bar graph
    #[arg(long)]
    pub no_graph: bool,
}

impl DaryaCli {
    /// Preprocess argv so that `-0`, `-1`, `-2` become `--0`, `--1`, `--2` for interface mode.
    /// Call this before parsing when using parse_from_iter for tests.
    pub fn preprocess_interface_args(
        args: impl IntoIterator<Item = std::ffi::OsString>,
    ) -> Vec<std::ffi::OsString> {
        args.into_iter()
            .map(|a| {
                let s = a.to_string_lossy();
                match s.as_ref() {
                    "-0" => std::ffi::OsString::from("--0"),
                    "-1" => std::ffi::OsString::from("--1"),
                    "-2" => std::ffi::OsString::from("--2"),
                    _ => a,
                }
            })
            .collect()
    }

    /// Parse from the environment, with -0/-1/-2 preprocessing.
    pub fn try_parse() -> Result<Self, clap::Error> {
        let args: Vec<std::ffi::OsString> = env::args_os().collect();
        let program = args
            .first()
            .cloned()
            .unwrap_or_else(|| std::ffi::OsString::from("darya"));
        let rest = Self::preprocess_interface_args(args.into_iter().skip(1));
        Self::try_parse_from(std::iter::once(program).chain(rest))
    }

    /// Parse from the given iterator (for tests). Call preprocess_interface_args on the args first.
    pub fn parse_from_iter<I>(iter: I) -> Result<CliArgs, CliParseError>
    where
        I: IntoIterator<Item = std::ffi::OsString>,
    {
        let preprocessed = Self::preprocess_interface_args(iter);
        let raw = Self::try_parse_from(
            std::iter::once(std::ffi::OsString::from("darya")).chain(preprocessed),
        )
        .map_err(CliParseError::Clap)?;
        raw.into_cli_args()
    }

    /// Convert parsed CLI into the app's CliArgs (resolve root, merge exclude-from, etc.).
    pub fn into_cli_args(self) -> Result<CliArgs, CliParseError> {
        let root = self
            .root
            .map(Ok)
            .unwrap_or_else(|| env::current_dir().map_err(CliParseError::CurrentDir))?;

        let mut exclude_patterns = self.exclude;
        for path in &self.exclude_from {
            let patterns = read_patterns_from_file(path)?;
            exclude_patterns.extend(patterns);
        }

        let extended = self.extended && !self.no_extended;

        let same_fs_override = if self.cross_file_system {
            Some(false)
        } else if self.one_file_system {
            Some(true)
        } else {
            None
        };

        let cache_policy = if self.exclude_caches {
            Some(false)
        } else if self.include_caches {
            Some(true)
        } else {
            None
        };

        let kernfs_policy = if self.exclude_kernfs {
            Some(false)
        } else if self.include_kernfs {
            Some(true)
        } else {
            None
        };

        let follow_symlinks_override = if self.no_follow_symlinks {
            Some(false)
        } else if self.follow_symlinks {
            Some(true)
        } else {
            None
        };

        let interface_mode = if self.force_summary {
            InterfaceMode::Summary
        } else if self.force_progress {
            InterfaceMode::Progress
        } else {
            InterfaceMode::Tui
        };

        let mut display_options = DisplayOptions::default();
        if self.si {
            display_options.use_si = true;
        }
        if self.disk_usage {
            display_options.prefer_disk = true;
        }
        if self.apparent_size {
            display_options.prefer_disk = false;
        }
        if self.show_hidden {
            display_options.show_hidden = true;
        }
        if self.hide_hidden {
            display_options.show_hidden = false;
        }
        if self.show_itemcount {
            display_options.show_item_count = true;
        }
        if self.hide_itemcount {
            display_options.show_item_count = false;
        }
        if self.show_mtime {
            display_options.show_mtime = true;
        }
        if self.hide_mtime {
            display_options.show_mtime = false;
        }
        if self.show_percent {
            display_options.show_percent = true;
        }
        if self.hide_percent {
            display_options.show_percent = false;
        }
        if self.no_graph {
            display_options.show_graph = false;
        }

        if self.thread_count == Some(0) {
            return Err(CliParseError::InvalidThreadCount("0".to_string()));
        }
        match self.compress_level {
            Some(l) if l == 0 || l > 9 => {
                return Err(CliParseError::InvalidCompressionLevel(l.to_string()));
            }
            _ => {}
        }
        if self.export_block_size == Some(0) {
            return Err(CliParseError::InvalidExportBlockSize("0".to_string()));
        }

        let import_snapshot = self.import_snapshot.map(|s| {
            if s == "-" {
                SnapshotEndpoint::StdIo
            } else {
                SnapshotEndpoint::File(PathBuf::from(s))
            }
        });
        let export_json = self.export_json.map(|s| {
            if s == "-" {
                SnapshotEndpoint::StdIo
            } else {
                SnapshotEndpoint::File(PathBuf::from(s))
            }
        });
        let export_binary = self.export_binary.map(|s| {
            if s == "-" {
                SnapshotEndpoint::StdIo
            } else {
                SnapshotEndpoint::File(PathBuf::from(s))
            }
        });

        Ok(CliArgs {
            root,
            exclude_patterns,
            extended,
            import_snapshot,
            export_json,
            export_binary,
            ignore_config: self.ignore_config,
            same_fs_override,
            cache_policy,
            kernfs_policy,
            thread_count: self.thread_count,
            follow_symlinks_override,
            export_compress: self.compress,
            export_compress_level: self.compress_level,
            export_block_size: self.export_block_size,
            interface_mode,
            display_options,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceMode {
    Tui,
    Progress,
    Summary,
}

/// Represents validated CLI arguments when running the application.
#[derive(Debug)]
pub struct CliArgs {
    pub root: PathBuf,
    pub exclude_patterns: Vec<String>,
    pub extended: bool,
    pub import_snapshot: Option<SnapshotEndpoint>,
    pub export_json: Option<SnapshotEndpoint>,
    pub export_binary: Option<SnapshotEndpoint>,
    pub ignore_config: bool,
    pub same_fs_override: Option<bool>,
    pub cache_policy: Option<bool>,
    pub kernfs_policy: Option<bool>,
    pub thread_count: Option<usize>,
    pub follow_symlinks_override: Option<bool>,
    pub export_compress: bool,
    pub export_compress_level: Option<u32>,
    pub export_block_size: Option<usize>,
    pub interface_mode: InterfaceMode,
    pub display_options: DisplayOptions,
}

fn read_patterns_from_file(path: &Path) -> Result<Vec<String>, CliParseError> {
    let contents = fs::read_to_string(path)
        .map_err(|err| CliParseError::ExcludeFile(path.to_path_buf(), err))?;
    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect())
}

/// Errors that can occur while parsing CLI arguments.
#[derive(Debug, thiserror::Error)]
pub enum CliParseError {
    #[error("clap error: {0}")]
    Clap(#[from] clap::Error),
    #[error("failed to read exclude-from file {0}: {1}")]
    ExcludeFile(PathBuf, #[source] std::io::Error),
    #[error("unable to determine current directory: {0}")]
    CurrentDir(#[from] std::io::Error),
    #[error("invalid thread count: {0} (must be at least 1)")]
    InvalidThreadCount(String),
    #[error("invalid compression level: {0} (must be 1-9)")]
    InvalidCompressionLevel(String),
    #[error("invalid export block size: {0} (must be at least 1)")]
    InvalidExportBlockSize(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotEndpoint;
    use std::ffi::OsString;
    use std::io::Write;

    fn parse(args: Vec<OsString>) -> Result<CliArgs, CliParseError> {
        DaryaCli::parse_from_iter(args)
    }

    #[test]
    fn parse_help_flag_returns_error_with_help_message() {
        let err = parse(vec![OsString::from("--help")]).unwrap_err();
        assert!(matches!(err, CliParseError::Clap(_)));
        // clap exits with help when --help is used; try_parse returns Err
        if let CliParseError::Clap(e) = err {
            assert!(e.to_string().contains("help"));
        }
    }

    #[test]
    fn parse_version_flag_returns_error_with_version() {
        let err = parse(vec![OsString::from("-V")]).unwrap_err();
        assert!(matches!(err, CliParseError::Clap(_)));
    }

    #[test]
    fn parse_import_flag_sets_endpoint() {
        let args = vec![OsString::from("-f"), OsString::from("-")];
        let cli = parse(args).unwrap();
        assert!(matches!(cli.import_snapshot, Some(SnapshotEndpoint::StdIo)));
    }

    #[test]
    fn parse_export_flags_collect_endpoints() {
        let args = vec![
            OsString::from("-o"),
            OsString::from("export.json"),
            OsString::from("-O"),
            OsString::from("export.bin"),
        ];
        let cli = parse(args).unwrap();
        assert!(matches!(cli.export_json, Some(SnapshotEndpoint::File(_))));
        assert!(matches!(cli.export_binary, Some(SnapshotEndpoint::File(_))));
    }

    #[test]
    fn parse_one_file_system_sets_override() {
        let args = vec![OsString::from("-x")];
        let cli = parse(args).unwrap();
        assert_eq!(cli.same_fs_override, Some(true));
    }

    #[test]
    fn parse_cross_file_system_sets_override() {
        let args = vec![OsString::from("--cross-file-system")];
        let cli = parse(args).unwrap();
        assert_eq!(cli.same_fs_override, Some(false));
    }

    #[test]
    fn parse_follow_symlinks_sets_override() {
        let args = vec![OsString::from("-L")];
        let cli = parse(args).unwrap();
        assert_eq!(cli.follow_symlinks_override, Some(true));
    }

    #[test]
    fn parse_thread_count_sets_override() {
        let args = vec![OsString::from("-t"), OsString::from("3")];
        let cli = parse(args).unwrap();
        assert_eq!(cli.thread_count, Some(3));
    }

    #[test]
    fn parse_display_options_flags() {
        let args = vec![
            OsString::from("--si"),
            OsString::from("--disk-usage"),
            OsString::from("--show-hidden"),
            OsString::from("--show-itemcount"),
            OsString::from("--show-mtime"),
            OsString::from("--show-percent"),
            OsString::from("--no-graph"),
        ];
        let cli = parse(args).unwrap();
        assert!(cli.display_options.use_si);
        assert!(cli.display_options.prefer_disk);
        assert!(cli.display_options.show_hidden);
        assert!(cli.display_options.show_item_count);
        assert!(cli.display_options.show_mtime);
        assert!(cli.display_options.show_percent);
        assert!(!cli.display_options.show_graph);
    }

    #[test]
    fn parse_exclude_from_reads_patterns() {
        let path = std::env::temp_dir().join("darya-exclude.tmp");
        let mut file = std::fs::File::create(&path).unwrap();
        std::io::Write::write_all(&mut file, b"ignored\n").unwrap();
        std::io::Write::write_all(&mut file, b"foo\n").unwrap();
        std::io::Write::write_all(&mut file, b"# comment\n").unwrap();
        file.flush().unwrap();

        let args = vec![
            OsString::from("-X"),
            OsString::from(path.to_string_lossy().into_owned()),
        ];
        let cli = parse(args).unwrap();
        assert_eq!(cli.exclude_patterns, vec!["ignored", "foo"]);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn parse_extended_flag_sets_mode() {
        let args = vec![OsString::from("-e"), OsString::from("/tmp")];
        let cli = parse(args).unwrap();
        assert!(cli.extended);
    }

    #[test]
    fn parse_no_extended_flag_unsets_mode() {
        let args = vec![OsString::from("--no-extended"), OsString::from("/tmp")];
        let cli = parse(args).unwrap();
        assert!(!cli.extended);
    }

    #[test]
    fn parse_ignore_config_presets_flag() {
        let args = vec![OsString::from("--ignore-config")];
        let cli = parse(args).unwrap();
        assert!(cli.ignore_config);
    }

    #[test]
    fn parse_export_tuning_flags() {
        let args = vec![
            OsString::from("-c"),
            OsString::from("--compress-level"),
            OsString::from("5"),
            OsString::from("--export-block-size"),
            OsString::from("16384"),
        ];
        let cli = parse(args).unwrap();
        assert!(cli.export_compress);
        assert_eq!(cli.export_compress_level, Some(5));
        assert_eq!(cli.export_block_size, Some(16384));
    }

    #[test]
    fn parse_interface_mode_progress() {
        let args = vec![OsString::from("-1")];
        let cli = parse(args).unwrap();
        assert_eq!(cli.interface_mode, InterfaceMode::Progress);
    }

    #[test]
    fn parse_interface_mode_summary() {
        let args = vec![OsString::from("-2")];
        let cli = parse(args).unwrap();
        assert_eq!(cli.interface_mode, InterfaceMode::Summary);
    }
}
