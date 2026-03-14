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
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use walkdir::WalkDir;

/// Errors that may occur while computing directory sizes.
#[derive(Debug, Error)]
pub enum SizeError {
    #[error("failed to walk {path}: {source}")]
    Walk {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
    #[error("failed to read metadata for {path}: {source}")]
    Metadata {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
}

/// Normalize a path so that `.` and redundant separators are resolved when possible.
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Compute the total size of files reachable from `path`, similar to `du` behavior.
pub fn total_size(path: &Path, follow_symlinks: bool) -> Result<u64, SizeError> {
    let walker = WalkDir::new(path).follow_links(follow_symlinks).into_iter();
    let mut total = 0;

    for entry in walker {
        let entry = entry.map_err(|source| SizeError::Walk {
            path: source
                .path()
                .map(PathBuf::from)
                .unwrap_or_else(|| path.to_path_buf()),
            source,
        })?;

        if entry.file_type().is_file() {
            let metadata = entry.metadata().map_err(|io_err| SizeError::Metadata {
                path: entry.path().to_path_buf(),
                source: io_err,
            })?;
            total += metadata.len();
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn create_temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "{name}-{ts}",
            ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            name = name
        ));
        let _ = fs::create_dir_all(&base);
        base
    }

    fn write_file(path: &Path, size: usize) {
        let mut file = File::create(path).expect("could not create test file");
        file.write_all(&vec![0u8; size]).expect("write failed");
    }

    #[test]
    fn normalize_path_resolves_components() {
        let base = create_temp_dir("dar-size");
        let subdir = base.join("sub");
        fs::create_dir_all(&subdir).expect("create subdir");
        let nested = subdir.join("..");
        let resolved = normalize_path(&nested);
        let canonical = fs::canonicalize(&base).expect("should canonicalize");
        assert_eq!(resolved, canonical);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn total_size_sums_files() {
        let base = create_temp_dir("dar-size-sum");
        let file_a = base.join("a.txt");
        let file_b = base.join("nested/b.txt");
        fs::create_dir_all(file_b.parent().unwrap()).unwrap();
        write_file(&file_a, 8);
        write_file(&file_b, 4);

        let total = total_size(&base, false).expect("size calculation failed");
        assert_eq!(total, 12);

        let _ = fs::remove_dir_all(&base);
    }
}
