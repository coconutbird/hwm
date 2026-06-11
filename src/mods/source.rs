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

    /// Read a file from the mod source
    pub fn read_file(&self, relative_path: &str) -> Result<Vec<u8>, ModError> {
        match self {
            Self::Directory(base) => {
                let full_path = base.join(relative_path);
                fs::read(&full_path).map_err(|e| ModError::Io(e, full_path))
            }
            Self::Zip(zip_path) => {
                let file = File::open(zip_path).map_err(|e| ModError::Io(e, zip_path.clone()))?;
                let mut archive = ZipArchive::new(BufReader::new(file))?;

                // Try exact path first, then try with forward slashes
                let normalized = relative_path.replace('\\', "/");

                // Find the file index (case-insensitive fallback)
                let file_index = if archive.by_name(&normalized).is_ok() {
                    None // Use by_name directly
                } else {
                    // Case-insensitive search
                    let mut found = None;
                    for i in 0..archive.len() {
                        if let Ok(f) = archive.by_index(i) {
                            if f.name().eq_ignore_ascii_case(&normalized) {
                                found = Some(i);
                                break;
                            }
                        }
                    }
                    found
                };

                let mut zip_file = match file_index {
                    Some(idx) => archive.by_index(idx)?,
                    None => archive.by_name(&normalized)?,
                };

                let mut contents = Vec::new();
                zip_file
                    .read_to_end(&mut contents)
                    .map_err(|e| ModError::Io(e, zip_path.clone()))?;
                Ok(contents)
            }
        }
    }

    /// Check if a file exists in the mod source
    pub fn file_exists(&self, relative_path: &str) -> bool {
        match self {
            Self::Directory(base) => base.join(relative_path).exists(),
            Self::Zip(zip_path) => {
                if let Ok(file) = File::open(zip_path) {
                    if let Ok(mut archive) = ZipArchive::new(BufReader::new(file)) {
                        let normalized = relative_path.replace('\\', "/");
                        return archive.by_name(&normalized).is_ok();
                    }
                }
                false
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
