//! Mod loading and management functionality
//!
//! This module provides support for loading and viewing mod content from various formats.
//! Currently supported formats:
//! - hwmod: Firebase/HaloWarsModding format (directories or zip files with manifest.xml)

mod error;
mod hwmod;
mod source;

pub use error::ModError;
pub use hwmod::HwMod;

use std::path::Path;

/// Load a mod from a path (auto-detects format)
pub fn load_mod(path: &Path) -> Result<HwMod, ModError> {
    // For now, we only support hwmod format
    // In the future, we can detect format based on file extension or content
    hwmod::load(path)
}
