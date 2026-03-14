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

use crate::snapshot::SnapshotEndpoint;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

const VERSION_TEXT: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));
const HELP_TEXT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " - ncdu-inspired disk usage explorer\n",
    "See https://dev.yorhel.nl/ncdu/man for the reference ncdu manual.\n\n",
    "USAGE:\n    ",
    env!("CARGO_PKG_NAME"),
    " [PATH]\n\n",
    "ARGS:\n",
    "    PATH        Optional starting directory (default: current working directory)\n\n",
    "OPTIONS:\n",
    "    -h, --help  Print this help screen\n",
    "    -v, --version  Print the version information\n",
    "    -f FILE  Import snapshot from FILE (use '-' for stdin, JSON only)\n",
    "    -o FILE  Export scan tree to FILE in JSON format\n",
    "    -O FILE  Export scan tree to FILE in binary format\n",
    "    -e, --extended  Enable extended metadata mode for owner/permissions/mtime\n",
    "    --no-extended  Disable extended metadata mode\n",
    "    -x, --one-file-system  Stay on the starting filesystem\n",
    "    --cross-file-system  Allow crossing filesystem boundaries\n",
    "    -L, --follow-symlinks  Follow symbolic links while scanning\n",
    "    --no-follow-symlinks  Do not follow symlinks\n",
    "    --include-caches  Explicitly include cache directories\n",
    "    --exclude-caches  Skip directories named cache\n",
    "    --include-kernfs  Include kernfs-mounted directories\n",
    "    --exclude-kernfs  Skip kernfs namespaces\n",
    "    -t N  Limit the runtime worker threads to N\n",
    "    -c, --compress  Compress the exported JSON with gzip\n",
    "    --compress-level N  Set gzip compression level (1-9)\n",
    "    --export-block-size BYTES  Set buffered block size for exports\n",
    "    --ignore-config  Do not load configuration files\n",
    "    --exclude PATTERN  Exclude files/directories by glob pattern (repeatable)\n"
);

/// Represents the CLI command to run.
#[derive(Debug)]
pub enum CliCommand {
    Run(CliArgs),
    Help,
    Version,
}

impl CliCommand {
    /// Parse the arguments coming from the environment.
    pub fn parse() -> Result<Self, CliParseError> {
        Self::parse_from_iter(env::args_os().skip(1))
    }

    fn parse_from_iter<I>(iter: I) -> Result<Self, CliParseError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let args: Vec<OsString> = iter.into_iter().collect();
        if args.iter().any(|arg| is_help_flag(arg)) {
            return Ok(Self::Help);
        }
        if args.iter().any(|arg| is_version_flag(arg)) {
            return Ok(Self::Version);
        }

        let mut args = args.into_iter().peekable();
        let mut root: Option<PathBuf> = None;
        let mut exclude_patterns = Vec::new();
        let mut import_snapshot = None;
        let mut export_json = None;
        let mut export_binary = None;
        let mut extended = false;
        let mut ignore_config = false;
        let mut same_fs_override: Option<bool> = None;
        let mut cache_policy: Option<bool> = None;
        let mut kernfs_policy: Option<bool> = None;
        let mut thread_count: Option<usize> = None;
        let mut follow_override: Option<bool> = None;
        let mut export_compress = false;
        let mut export_compress_level = None;
        let mut export_block_size = None;

        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--exclude") => {
                    let value = take_option_value(&mut args, "--exclude")?;
                    exclude_patterns.push(value.to_string_lossy().into_owned());
                }
                Some("-X") | Some("--exclude-from") => {
                    let value = take_option_value(&mut args, arg.to_str().unwrap())?;
                    let path = PathBuf::from(value);
                    let patterns = read_patterns_from_file(&path)?;
                    exclude_patterns.extend(patterns);
                }
                Some("-c") | Some("--compress") => {
                    export_compress = true;
                }
                Some("--compress-level") => {
                    let value = take_option_value(&mut args, "--compress-level")?;
                    export_compress_level = Some(parse_compression_level(&value)?);
                }
                Some("--export-block-size") => {
                    let value = take_option_value(&mut args, "--export-block-size")?;
                    export_block_size = Some(parse_block_size(&value)?);
                }
                Some("-f") => {
                    let value = take_option_value(&mut args, "-f")?;
                    import_snapshot = Some(parse_endpoint(&value));
                }
                Some("-o") => {
                    let value = take_option_value(&mut args, "-o")?;
                    export_json = Some(parse_endpoint(&value));
                }
                Some("-O") => {
                    let value = take_option_value(&mut args, "-O")?;
                    export_binary = Some(parse_endpoint(&value));
                }
                Some("-e") | Some("--extended") => {
                    extended = true;
                }
                Some("--no-extended") => {
                    extended = false;
                }
                Some("-x") | Some("--one-file-system") => {
                    same_fs_override = Some(true);
                }
                Some("--cross-file-system") => {
                    same_fs_override = Some(false);
                }
                Some("--include-caches") => {
                    cache_policy = Some(true);
                }
                Some("--exclude-caches") => {
                    cache_policy = Some(false);
                }
                Some("--include-kernfs") => {
                    kernfs_policy = Some(true);
                }
                Some("--exclude-kernfs") => {
                    kernfs_policy = Some(false);
                }
                Some("-t") => {
                    let value = take_option_value(&mut args, "-t")?;
                    thread_count = Some(parse_thread_count(&value)?);
                }
                Some("-L") | Some("--follow-symlinks") => {
                    follow_override = Some(true);
                }
                Some("--no-follow-symlinks") => {
                    follow_override = Some(false);
                }
                Some("--ignore-config") => {
                    ignore_config = true;
                }
                Some(value) if value.starts_with('-') => {
                    return Err(CliParseError::UnknownOption(value.to_string()));
                }
                _ => {
                    if root.is_none() {
                        root = Some(PathBuf::from(arg));
                    } else {
                        return Err(CliParseError::TooManyArguments);
                    }
                }
            }
        }

        let cli = if let Some(root) = root {
            CliArgs {
                root,
                exclude_patterns,
                extended,
                import_snapshot,
                export_json,
                export_binary,
                ignore_config,
                same_fs_override,
                cache_policy,
                kernfs_policy,
                thread_count,
                follow_symlinks_override: follow_override,
                export_compress,
                export_compress_level,
                export_block_size,
            }
        } else {
            let mut cli = CliArgs::from_current_dir()?;
            cli.exclude_patterns = exclude_patterns;
            cli.extended = extended;
            cli.import_snapshot = import_snapshot;
            cli.export_json = export_json;
            cli.export_binary = export_binary;
            cli.ignore_config = ignore_config;
            cli.same_fs_override = same_fs_override;
            cli.cache_policy = cache_policy;
            cli.kernfs_policy = kernfs_policy;
            cli.thread_count = thread_count;
            cli.follow_symlinks_override = follow_override;
            cli.export_compress = export_compress;
            cli.export_compress_level = export_compress_level;
            cli.export_block_size = export_block_size;
            cli
        };
        Ok(Self::Run(cli))
    }

    /// Return the help text displayed when `--help` is requested.
    pub fn help_text() -> &'static str {
        HELP_TEXT
    }

    pub fn version_text() -> &'static str {
        VERSION_TEXT
    }
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
}

impl CliArgs {
    fn from_current_dir() -> Result<Self, CliParseError> {
        let root = env::current_dir().map_err(CliParseError::CurrentDir)?;
        Ok(Self {
            root,
            exclude_patterns: Vec::new(),
            extended: false,
            import_snapshot: None,
            export_json: None,
            export_binary: None,
            ignore_config: false,
            same_fs_override: None,
            cache_policy: None,
            kernfs_policy: None,
            thread_count: None,
            follow_symlinks_override: None,
            export_compress: false,
            export_compress_level: None,
            export_block_size: None,
        })
    }
}

fn take_option_value<I>(args: &mut I, flag: &str) -> Result<OsString, CliParseError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .ok_or_else(|| CliParseError::MissingOptionValue(flag.to_string()))
}

fn parse_endpoint(value: &OsStr) -> SnapshotEndpoint {
    if value == OsStr::new("-") {
        SnapshotEndpoint::StdIo
    } else {
        SnapshotEndpoint::File(PathBuf::from(value))
    }
}

fn parse_compression_level(value: &OsStr) -> Result<u32, CliParseError> {
    let text = value.to_string_lossy();
    let level = text
        .parse::<u32>()
        .map_err(|_| CliParseError::InvalidCompressionLevel(text.to_string()))?;
    if level > 9 {
        return Err(CliParseError::InvalidCompressionLevel(text.to_string()));
    }
    Ok(level)
}

fn parse_block_size(value: &OsStr) -> Result<usize, CliParseError> {
    let text = value.to_string_lossy();
    let size = text
        .parse::<usize>()
        .map_err(|_| CliParseError::InvalidExportBlockSize(text.to_string()))?;
    if size == 0 {
        return Err(CliParseError::InvalidExportBlockSize(text.to_string()));
    }
    Ok(size)
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

fn parse_thread_count(value: &OsStr) -> Result<usize, CliParseError> {
    let text = value.to_string_lossy().to_string();
    let parsed = text
        .parse::<usize>()
        .map_err(|_| CliParseError::InvalidThreadCount(text.clone()))?;
    if parsed == 0 {
        return Err(CliParseError::InvalidThreadCount(text));
    }
    Ok(parsed)
}

fn is_help_flag(arg: &OsStr) -> bool {
    matches!(arg.to_str(), Some("--help") | Some("-h"))
}

fn is_version_flag(arg: &OsStr) -> bool {
    matches!(arg.to_str(), Some("-v") | Some("-V") | Some("--version"))
}

/// Errors that can occur while parsing CLI arguments.
#[derive(Debug, thiserror::Error)]
pub enum CliParseError {
    #[error("too many arguments were provided")]
    TooManyArguments,
    #[error("unknown option: {0}")]
    UnknownOption(String),
    #[error("missing value for option: {0}")]
    MissingOptionValue(String),
    #[error("invalid thread count: {0}")]
    InvalidThreadCount(String),
    #[error("invalid compression level: {0}")]
    InvalidCompressionLevel(String),
    #[error("invalid export block size: {0}")]
    InvalidExportBlockSize(String),
    #[error("failed to read exclude-from file {0}: {1}")]
    ExcludeFile(PathBuf, #[source] std::io::Error),
    #[error("unable to determine current directory: {0}")]
    CurrentDir(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotEndpoint;
    use std::ffi::OsString;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn parse_help_flag_returns_help() {
        assert!(matches!(
            CliCommand::parse_from_iter(vec![OsString::from("--help")]),
            Ok(CliCommand::Help)
        ));
    }

    #[test]
    fn parse_version_flag_returns_version() {
        assert!(matches!(
            CliCommand::parse_from_iter(vec![OsString::from("-V")]),
            Ok(CliCommand::Version)
        ));
    }

    #[test]
    fn parse_import_flag_sets_endpoint() {
        let args = vec![OsString::from("-f"), OsString::from("-")];
        assert!(
            matches!(CliCommand::parse_from_iter(args), Ok(CliCommand::Run(cli)) if matches!(cli.import_snapshot, Some(SnapshotEndpoint::StdIo)))
        );
    }

    #[test]
    fn parse_export_flags_collect_endpoints() {
        let args = vec![
            OsString::from("-o"),
            OsString::from("export.json"),
            OsString::from("-O"),
            OsString::from("export.bin"),
        ];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert!(matches!(cli.export_json, Some(SnapshotEndpoint::File(_))));
            assert!(matches!(cli.export_binary, Some(SnapshotEndpoint::File(_))));
        } else {
            panic!("expected run command");
        }
    }

    #[test]
    fn parse_one_file_system_sets_override() {
        let args = vec![OsString::from("-x")];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert_eq!(cli.same_fs_override, Some(true));
        } else {
            panic!("expected run command");
        }
    }

    #[test]
    fn parse_cross_file_system_sets_override() {
        let args = vec![OsString::from("--cross-file-system")];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert_eq!(cli.same_fs_override, Some(false));
        } else {
            panic!("expected run command");
        }
    }

    #[test]
    fn parse_follow_symlinks_sets_override() {
        let args = vec![OsString::from("-L")];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert_eq!(cli.follow_symlinks_override, Some(true));
        } else {
            panic!("expected run command");
        }
    }

    #[test]
    fn parse_thread_count_sets_override() {
        let args = vec![OsString::from("-t"), OsString::from("3")];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert_eq!(cli.thread_count, Some(3));
        } else {
            panic!("expected run command");
        }
    }

    #[test]
    fn parse_exclude_from_reads_patterns() {
        let path = std::env::temp_dir().join("dar-exclude.tmp");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "ignored").unwrap();
        writeln!(file, "foo").unwrap();
        writeln!(file, "# comment").unwrap();
        file.flush().unwrap();

        let args = vec![
            OsString::from("-X"),
            OsString::from(path.to_string_lossy().into_owned()),
        ];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert_eq!(cli.exclude_patterns, vec!["ignored", "foo"]);
        } else {
            panic!("expected run command");
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn parse_extended_flag_sets_mode() {
        let args = vec![OsString::from("-e"), OsString::from("/tmp")];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert!(cli.extended);
        } else {
            panic!("expected run command");
        }
    }

    #[test]
    fn parse_no_extended_flag_unsets_mode() {
        let args = vec![OsString::from("--no-extended"), OsString::from("/tmp")];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert!(!cli.extended);
        } else {
            panic!("expected run command");
        }
    }

    #[test]
    fn parse_ignore_config_presets_flag() {
        let args = vec![OsString::from("--ignore-config")];
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert!(cli.ignore_config);
        } else {
            panic!("expected run command");
        }
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
        if let Ok(CliCommand::Run(cli)) = CliCommand::parse_from_iter(args) {
            assert!(cli.export_compress);
            assert_eq!(cli.export_compress_level, Some(5));
            assert_eq!(cli.export_block_size, Some(16384));
        } else {
            panic!("expected run command");
        }
    }
}
