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
use std::path::PathBuf;

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

        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--exclude") => {
                    let value = take_option_value(&mut args, "--exclude")?;
                    exclude_patterns.push(value.to_string_lossy().into_owned());
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
            }
        } else {
            let mut cli = CliArgs::from_current_dir()?;
            cli.exclude_patterns = exclude_patterns;
            cli.extended = extended;
            cli.import_snapshot = import_snapshot;
            cli.export_json = export_json;
            cli.export_binary = export_binary;
            cli.ignore_config = ignore_config;
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
    #[error("unable to determine current directory: {0}")]
    CurrentDir(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotEndpoint;
    use std::ffi::OsString;

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
}
