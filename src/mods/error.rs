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
    /// No `.hwmod` manifest was found in the given directory
    ManifestNotFound(PathBuf),
}

impl std::fmt::Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(path) => write!(f, "Mod not found: {}", path.display()),
            Self::Io(e, path) => write!(f, "IO error reading {}: {}", path.display(), e),
            Self::ManifestParse(e) => write!(f, "Failed to parse manifest: {}", e),
            Self::ManifestNotFound(dir) => {
                write!(f, "No .hwmod manifest found in {}", dir.display())
            }
        }
    }
}

impl std::error::Error for ModError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e, _) => Some(e),
            _ => None,
        }
    }
}
