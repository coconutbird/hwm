use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

/// Game identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Game {
    /// Halo Wars Definitive Edition
    HaloWars1,
    /// Not yet supported - Xbox Store app
    HaloWars2,
}

impl FromStr for Game {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hw1" | "halowar1" | "halowars1" => Ok(Game::HaloWars1),
            "hw2" | "halowars2" | "halowar2" => Ok(Game::HaloWars2),
            _ => Err(format!("Unknown game '{}'. Use 'hw1' or 'hw2'", s)),
        }
    }
}

/// Configuration for a single game installation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameConfig {
    /// Path to the game installation directory
    pub path: Option<PathBuf>,
    /// Whether this path was auto-detected or manually set
    #[serde(default)]
    pub auto_detected: bool,
    /// Path to the active mod directory
    pub mod_path: Option<PathBuf>,
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub halo_wars_1: GameConfig,
    pub halo_wars_2: GameConfig,
}

impl Config {
    /// Load config from the default config file location
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::Io(e, config_path.clone()))?;

        toml::from_str(&contents).map_err(ConfigError::Parse)
    }

    /// Save config to the default config file location
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::Io(e, parent.to_path_buf()))?;
        }

        let contents = toml::to_string_pretty(self).map_err(ConfigError::Serialize)?;

        std::fs::write(&config_path, contents)
            .map_err(|e| ConfigError::Io(e, config_path.clone()))?;

        Ok(())
    }

    /// Get the config file path (next to the executable)
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let exe_path = std::env::current_exe()
            .map_err(|e| ConfigError::Io(e, PathBuf::from("current_exe")))?;

        let exe_dir = exe_path.parent().ok_or(ConfigError::NoConfigDir)?;

        Ok(exe_dir.join("config.toml"))
    }

    /// Get config for a specific game
    pub fn game_config(&self, game: Game) -> &GameConfig {
        match game {
            Game::HaloWars1 => &self.halo_wars_1,
            Game::HaloWars2 => &self.halo_wars_2,
        }
    }

    /// Get mutable config for a specific game
    pub fn game_config_mut(&mut self, game: Game) -> &mut GameConfig {
        match game {
            Game::HaloWars1 => &mut self.halo_wars_1,
            Game::HaloWars2 => &mut self.halo_wars_2,
        }
    }

    /// Set game path manually
    pub fn set_game_path(&mut self, game: Game, path: PathBuf) {
        let config = self.game_config_mut(game);
        config.path = Some(path);
        config.auto_detected = false;
    }
}

#[derive(Debug)]
pub enum ConfigError {
    NoConfigDir,
    Io(std::io::Error, PathBuf),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoConfigDir => write!(f, "Could not determine config directory"),
            Self::Io(e, path) => write!(f, "IO error at {}: {}", path.display(), e),
            Self::Parse(e) => write!(f, "Failed to parse config: {}", e),
            Self::Serialize(e) => write!(f, "Failed to serialize config: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}
