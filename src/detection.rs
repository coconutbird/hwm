use std::path::{Path, PathBuf};

use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

use crate::config::Game;

/// Known game executable names
const HW1_EXECUTABLE: &str = "xgameFinal.exe";

/// Steam App ID for Halo Wars: Definitive Edition
const HW1_STEAM_APP_ID: &str = "459220";

/// Detect a specific game installation
pub fn detect_game(game: Game) -> Option<PathBuf> {
    match game {
        Game::HaloWars1 => detect_halo_wars_1(),
        Game::HaloWars2 => None, // Not yet supported - Xbox Store app
    }
}

/// Detect Halo Wars 1 (Definitive Edition) installation via Windows Registry
fn detect_halo_wars_1() -> Option<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key_path = format!(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App {}",
        HW1_STEAM_APP_ID
    );

    let key = hklm.open_subkey(&key_path).ok()?;
    let install_path: String = key.get_value("InstallLocation").ok()?;
    let path = PathBuf::from(install_path);

    if is_valid_hw1_install(&path) {
        Some(path)
    } else {
        None
    }
}

/// Validate a Halo Wars 1 installation
fn is_valid_hw1_install(path: &Path) -> bool {
    path.exists() && path.join(HW1_EXECUTABLE).exists()
}
