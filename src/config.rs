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

use directories::ProjectDirs;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Represents the parsed configuration file and the default values used when no file is present.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub sorting: SortingConfig,
    pub scan: ScanConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub show_bars: bool,
    pub show_hidden: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SortingConfig {
    pub mode: SortMode,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ScanConfig {
    pub follow_symlinks: bool,
    pub max_depth: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortMode {
    #[default]
    SizeDesc,
    SizeAsc,
    Name,
    ModifiedTime,
}

/// Result of loading the configuration file.
#[derive(Debug)]
pub struct ConfigLoad {
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub error: Option<ConfigError>,
}

impl ConfigLoad {
    pub fn error(&self) -> Option<&ConfigError> {
        self.error.as_ref()
    }

    pub fn source_description(&self) -> String {
        match self.config_path.as_deref() {
            Some(path) => format!("from {path}", path = path.display()),
            None => "defaults".into(),
        }
    }
}

/// Errors that occur while reading the configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// Load the configuration, returning defaults when the file is absent or invalid.
pub fn load() -> ConfigLoad {
    let config_path = config_file_path();
    let mut load = ConfigLoad {
        config: Config::default(),
        config_path: config_path.clone(),
        error: None,
    };

    if let Some(path) = config_path.as_deref().filter(|path| path.exists()) {
        match parse_config_file(path) {
            Ok(config) => load.config = config,
            Err(err) => load.error = Some(err),
        }
    }

    load
}

fn config_file_path() -> Option<PathBuf> {
    ProjectDirs::from("org", "dar", "dar").map(|dirs| dirs.config_dir().join("config.toml"))
}

fn parse_config_file(path: &Path) -> Result<Config, ConfigError> {
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&contents).map_err(|source| ConfigError::Decode {
        path: path.to_path_buf(),
        source,
    })
}
