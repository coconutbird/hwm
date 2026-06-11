//! Error types for mod loading

use std::path::PathBuf;

/// Errors that can occur when loading or working with mods
#[derive(Debug)]
pub enum ModError {
    /// The specified path does not exist
    NotFound(PathBuf),
    /// Failed to read a file
    Io(std::io::Error, PathBuf),
    /// Failed to parse the mod manifest XML
    ManifestParse(String),
    /// Failed to read from zip archive
    Zip(zip::result::ZipError),
    /// Manifest file not found in mod
    ManifestNotFound,
    /// Unsupported mod format
    UnsupportedFormat(String),
    /// A mod-relative path was absolute or escaped the mod root
    InvalidPath(String),
}

impl std::fmt::Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(path) => write!(f, "Mod not found: {}", path.display()),
            Self::Io(e, path) => write!(f, "IO error reading {}: {}", path.display(), e),
            Self::ManifestParse(e) => write!(f, "Failed to parse manifest: {}", e),
            Self::Zip(e) => write!(f, "Zip error: {}", e),
            Self::ManifestNotFound => write!(f, "Manifest file not found in mod"),
            Self::UnsupportedFormat(fmt) => write!(f, "Unsupported mod format: {}", fmt),
            Self::InvalidPath(p) => write!(f, "Invalid mod-relative path: {}", p),
        }
    }
}

impl std::error::Error for ModError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e, _) => Some(e),
            Self::Zip(e) => Some(e),
            _ => None,
        }
    }
}

impl From<zip::result::ZipError> for ModError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Zip(e)
    }
}
