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
    pub exclude_patterns: Vec<String>,
    pub count_hard_links_once: bool,
    pub one_file_system: bool,
    pub exclude_caches: bool,
    pub exclude_kernfs: bool,
    pub thread_count: Option<usize>,
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

fn config_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = system_config_path() {
        paths.push(path);
    }
    if let Some(path) = ncdu_config_path() {
        paths.push(path);
    }
    if let Some(path) = config_file_path() {
        paths.push(path);
    }
    paths
}

fn system_config_path() -> Option<PathBuf> {
    let path = PathBuf::from("/etc/dar/config");
    if path.exists() { Some(path) } else { None }
}

fn ncdu_config_path() -> Option<PathBuf> {
    let path = PathBuf::from("/etc/ncdu.conf");
    if path.exists() { Some(path) } else { None }
}

/// Load the configuration, returning defaults when the file is absent or invalid.
pub fn load(ignore_config: bool) -> ConfigLoad {
    let mut load = ConfigLoad {
        config: Config::default(),
        config_path: None,
        error: None,
    };

    if ignore_config {
        return load;
    }

    for path in config_file_paths() {
        match parse_config_file(&path) {
            Ok(config) => {
                load.config = config;
                load.config_path = Some(path);
                load.error = None;
            }
            Err(err) => {
                if load.error.is_none() {
                    load.error = Some(err);
                    load.config_path = Some(path);
                }
            }
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
