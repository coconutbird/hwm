//! Mod loading and management functionality
//!
//! This module loads and inspects mods in the **hwmod** format
//! (Firebase / HaloWarsModding): a mod is a directory containing a `.hwmod`
//! XML manifest alongside a `ModData` content folder. Either the directory or
//! the `.hwmod` manifest file itself can be passed in.

mod error;
mod hwmod;

pub use error::ModError;
pub use hwmod::HwMod;

use std::path::Path;

/// Load a mod from a path (auto-detects format)
pub fn load_mod(path: &Path) -> Result<HwMod, ModError> {
    // For now, hwmod is the only supported format.
    hwmod::load(path)
}
