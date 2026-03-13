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

use std::{env, ffi::OsStr, path::PathBuf};

const HELP_TEXT: &str = "\
dar [PATH]
Explore disk usage interactively from the terminal.

USAGE:
    dar [PATH]

ARGS:
    PATH        Optional starting directory (default: current working directory)

OPTIONS:
    -h, --help  Print this help screen
";

/// Represents the CLI command to run.
#[derive(Debug)]
pub enum CliCommand {
    Run(CliArgs),
    Help,
}

impl CliCommand {
    /// Parse the arguments coming from the environment.
    pub fn parse() -> Result<Self, CliParseError> {
        let mut args = env::args_os().skip(1);
        match args.next() {
            None => Ok(Self::Run(CliArgs::from_current_dir()?)),
            Some(first) if is_help_flag(&first) => {
                if args.next().is_some() {
                    return Err(CliParseError::TooManyArguments);
                }
                Ok(Self::Help)
            }
            Some(first) => {
                if is_unknown_flag(&first) {
                    return Err(CliParseError::UnknownOption(
                        first.to_string_lossy().into_owned(),
                    ));
                }
                if args.next().is_some() {
                    return Err(CliParseError::TooManyArguments);
                }
                Ok(Self::Run(CliArgs {
                    root: PathBuf::from(first),
                }))
            }
        }
    }

    /// Return the help text displayed when `--help` is requested.
    pub fn help_text() -> &'static str {
        HELP_TEXT
    }
}

/// Represents validated CLI arguments when running the application.
#[derive(Debug)]
pub struct CliArgs {
    pub root: PathBuf,
}

impl CliArgs {
    fn from_current_dir() -> Result<Self, CliParseError> {
        let root = env::current_dir().map_err(CliParseError::CurrentDir)?;
        Ok(Self { root })
    }
}

fn is_help_flag(arg: &OsStr) -> bool {
    matches!(arg.to_str(), Some("--help") | Some("-h"))
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
    #[error("unable to determine current directory: {0}")]
    CurrentDir(#[from] std::io::Error),
}
