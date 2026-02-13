use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Game;

/// Game executable names
const HW1_EXECUTABLE: &str = "xgameFinal.exe";

/// Launch a game with optional mod
pub fn launch_game(
    game: Game,
    game_path: &Path,
    mod_path: Option<&PathBuf>,
) -> Result<(), LaunchError> {
    match game {
        Game::HaloWars1 => launch_halo_wars_1(game_path, mod_path),
        Game::HaloWars2 => Err(LaunchError::NotSupported(
            "Halo Wars 2 launch not yet supported".to_string(),
        )),
    }
}

/// Launch Halo Wars 1 (Definitive Edition)
fn launch_halo_wars_1(game_path: &Path, mod_path: Option<&PathBuf>) -> Result<(), LaunchError> {
    // Set up ModManifest.txt
    let mod_manifest_path = get_hw1_mod_manifest_path()?;

    // Ensure parent directory exists
    if let Some(parent) = mod_manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| LaunchError::Io(e, parent.to_path_buf()))?;
    }

    // Write mod path to ModManifest.txt (or clear it for vanilla)
    match mod_path {
        Some(path) => {
            let content = path.to_string_lossy().to_string();
            fs::write(&mod_manifest_path, &content)
                .map_err(|e| LaunchError::Io(e, mod_manifest_path.clone()))?;
        }
        None => {
            // Clear the manifest for vanilla play
            fs::write(&mod_manifest_path, "")
                .map_err(|e| LaunchError::Io(e, mod_manifest_path.clone()))?;
        }
    }

    // Launch the game executable
    let exe_path = game_path.join(HW1_EXECUTABLE);
    if !exe_path.exists() {
        return Err(LaunchError::ExecutableNotFound(exe_path));
    }

    Command::new(&exe_path)
        .current_dir(game_path)
        .spawn()
        .map_err(|e| LaunchError::Io(e, exe_path))?;

    Ok(())
}

/// Get the path to HW1's ModManifest.txt
fn get_hw1_mod_manifest_path() -> Result<PathBuf, LaunchError> {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|_| LaunchError::EnvVar("LOCALAPPDATA not set".to_string()))?;

    Ok(PathBuf::from(local_app_data)
        .join("Halo Wars")
        .join("ModManifest.txt"))
}

#[derive(Debug)]
pub enum LaunchError {
    NotSupported(String),
    ExecutableNotFound(PathBuf),
    Io(std::io::Error, PathBuf),
    EnvVar(String),
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSupported(msg) => write!(f, "{}", msg),
            Self::ExecutableNotFound(path) => write!(f, "Executable not found: {}", path.display()),
            Self::Io(e, path) => write!(f, "IO error at {}: {}", path.display(), e),
            Self::EnvVar(msg) => write!(f, "Environment variable error: {}", msg),
        }
    }
}

impl std::error::Error for LaunchError {}
