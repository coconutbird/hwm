//! Mod source abstraction for reading from directories or zip files

use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::mods::ModError;

/// A source for mod files - either a directory or a zip archive
pub enum ModSource {
    /// Mod stored as an unpacked directory
    Directory(PathBuf),
    /// Mod stored as a zip archive
    Zip(PathBuf),
}

impl ModSource {
    /// Create a ModSource from a path (auto-detects directory vs zip)
    pub fn from_path(path: &Path) -> Result<Self, ModError> {
        if !path.exists() {
            return Err(ModError::NotFound(path.to_path_buf()));
        }

        if path.is_dir() {
            Ok(Self::Directory(path.to_path_buf()))
        } else if path.is_file() {
            // Check if it's a zip file by extension or magic bytes
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("hwmod") {
                Ok(Self::Zip(path.to_path_buf()))
            } else {
                // Try to open as zip anyway
                match File::open(path) {
                    Ok(file) => match ZipArchive::new(BufReader::new(file)) {
                        Ok(_) => Ok(Self::Zip(path.to_path_buf())),
                        Err(_) => Err(ModError::UnsupportedFormat(
                            "Not a directory or valid zip file".to_string(),
                        )),
                    },
                    Err(e) => Err(ModError::Io(e, path.to_path_buf())),
                }
            }
        } else {
            Err(ModError::UnsupportedFormat(
                "Path is neither a file nor directory".to_string(),
            ))
        }
    }

    /// Read a file from the mod source.
    ///
    /// `relative_path` must stay within the mod root: absolute paths and
    /// parent-directory (`..`) components are rejected so a manifest can't
    /// point the reader at arbitrary files on disk.
    pub fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, ModError> {
        if !is_safe_relative_path(relative_path) {
            return Err(ModError::InvalidPath(relative_path.to_string()));
        }

        match self {
            Self::Directory(base) => {
                let full_path = base.join(relative_path);
                fs::read(&full_path).map_err(|e| ModError::Io(e, full_path))
            }
            Self::Zip(zip_path) => {
                let file = File::open(zip_path).map_err(|e| ModError::Io(e, zip_path.clone()))?;
                let mut archive = ZipArchive::new(BufReader::new(file))?;

                let normalized = relative_path.replace('\\', "/");
                let index = zip_entry_index(&mut archive, &normalized)
                    .ok_or(ModError::Zip(zip::result::ZipError::FileNotFound))?;

                let mut zip_file = archive.by_index(index)?;
                let mut contents = Vec::new();
                zip_file
                    .read_to_end(&mut contents)
                    .map_err(|e| ModError::Io(e, zip_path.clone()))?;
                Ok(contents)
            }
        }
    }

    /// Get the path to this mod source
    pub fn path(&self) -> &Path {
        match self {
            Self::Directory(p) | Self::Zip(p) => p,
        }
    }
}

/// Reject absolute paths and any `..` component so a mod-supplied relative
/// path can't escape the mod root (zip-slip / path traversal).
fn is_safe_relative_path(relative_path: &str) -> bool {
    let normalized = relative_path.replace('\\', "/");
    if normalized.is_empty() || normalized.starts_with('/') || Path::new(&normalized).is_absolute()
    {
        return false;
    }
    !normalized.split('/').any(|component| component == "..")
}

/// Find a zip entry by name in a single pass: an exact match wins immediately,
/// otherwise the first case-insensitive match is used.
fn zip_entry_index(archive: &mut ZipArchive<BufReader<File>>, name: &str) -> Option<usize> {
    let mut fallback = None;
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        let entry_name = entry.name();
        if entry_name == name {
            return Some(i);
        }
        if fallback.is_none() && entry_name.eq_ignore_ascii_case(name) {
            fallback = Some(i);
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_relative_paths() {
        assert!(is_safe_relative_path("manifest.xml"));
        assert!(is_safe_relative_path("art/banner.png"));
        assert!(is_safe_relative_path("art\\icon.png"));
    }

    #[test]
    fn rejects_escaping_paths() {
        assert!(!is_safe_relative_path(""));
        assert!(!is_safe_relative_path("../secret"));
        assert!(!is_safe_relative_path("art/../../secret"));
        assert!(!is_safe_relative_path("..\\secret"));
        assert!(!is_safe_relative_path("/etc/passwd"));
    }

    #[test]
    fn read_file_rejects_traversal() {
        // A directory source must refuse to read outside its root, before
        // ever touching the filesystem.
        let source = ModSource::Directory(std::env::temp_dir());
        let result = source.read_file("../anything");
        assert!(matches!(result, Err(ModError::InvalidPath(_))));
    }
}
