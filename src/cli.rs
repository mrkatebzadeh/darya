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

use std::{
    env,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

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

        while let Some(arg) = args.next() {
            if arg == "--exclude" {
                let Some(pattern) = args.next() else {
                    return Err(CliParseError::MissingOptionValue("--exclude".to_string()));
                };
                exclude_patterns.push(pattern.to_string_lossy().into_owned());
                continue;
            }

            if is_unknown_flag(&arg) {
                return Err(CliParseError::UnknownOption(
                    arg.to_string_lossy().into_owned(),
                ));
            }

            if root.is_none() {
                root = Some(PathBuf::from(arg));
            } else {
                return Err(CliParseError::TooManyArguments);
            }
        }

        let cli = if let Some(root) = root {
            CliArgs {
                root,
                exclude_patterns,
            }
        } else {
            let mut cli = CliArgs::from_current_dir()?;
            cli.exclude_patterns = exclude_patterns;
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
}

impl CliArgs {
    fn from_current_dir() -> Result<Self, CliParseError> {
        let root = env::current_dir().map_err(CliParseError::CurrentDir)?;
        Ok(Self {
            root,
            exclude_patterns: Vec::new(),
        })
    }
}

fn is_help_flag(arg: &OsStr) -> bool {
    matches!(arg.to_str(), Some("--help") | Some("-h"))
}

fn is_version_flag(arg: &OsStr) -> bool {
    matches!(arg.to_str(), Some("-v") | Some("-V") | Some("--version"))
}

fn is_unknown_flag(arg: &OsStr) -> bool {
    arg.to_str()
        .map(|value| value.starts_with('-'))
        .unwrap_or(false)
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
    fn parse_path_with_help_flag_prefers_help() {
        let args = vec![OsString::from("--help"), OsString::from("/tmp")];
        assert!(matches!(
            CliCommand::parse_from_iter(args),
            Ok(CliCommand::Help)
        ));
    }

    #[test]
    fn parse_path_with_version_flag_prefers_version() {
        let args = vec![OsString::from("--version"), OsString::from("/tmp")];
        assert!(matches!(
            CliCommand::parse_from_iter(args),
            Ok(CliCommand::Version)
        ));
    }
}
