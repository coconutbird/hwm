mod config;
mod detection;
mod launch;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use config::Game;

#[derive(Parser)]
#[command(name = "hwde-manager")]
#[command(about = "Halo Wars Definitive Edition mod manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Configuration commands
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    #[command(after_help = "Examples:
  Launch with configured mod:  hwde-manager launch hw1
  Launch vanilla (no mod):     hwde-manager launch hw1 --vanilla")]
    /// Launch a game with the configured mod
    Launch {
        /// Game to launch (hw1 or hw2)
        game: Game,
        /// Launch without any mod (clears ModManifest)
        #[arg(long)]
        vanilla: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration for all games
    Show,

    #[command(after_help = "Examples:
  Get current path:      hwde-manager config path hw1
  Auto-detect and save:  hwde-manager config path hw1 --detect
  Set path manually:     hwde-manager config path hw1 \"C:\\Games\\HW1\"
  Clear path:            hwde-manager config path hw1 --clear")]
    /// Get or set game installation path
    Path {
        /// Game (hw1 or hw2)
        game: Game,
        /// Auto-detect the game installation path and save it
        #[arg(long)]
        detect: bool,
        /// Clear the configured path
        #[arg(long)]
        clear: bool,
        /// Path to set manually (omit to get current path)
        path: Option<PathBuf>,
    },

    #[command(after_help = "Examples:
  Get current mod path:  hwde-manager config mod-path hw1
  Set mod path:          hwde-manager config mod-path hw1 \"C:\\Mods\\MyMod\"
  Clear mod path:        hwde-manager config mod-path hw1 --clear")]
    /// Get or set active mod path
    ModPath {
        /// Game (hw1 or hw2)
        game: Game,
        /// Clear the configured mod path
        #[arg(long)]
        clear: bool,
        /// Path to set (omit to get current path)
        path: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    // Load or create config
    let mut config = config::Config::load().unwrap_or_default();

    match &cli.command {
        Commands::Config { command } => handle_config_command(&mut config, command),
        Commands::Launch { game, vanilla } => handle_launch_command(&config, *game, *vanilla),
    }
}

fn handle_launch_command(config: &config::Config, game: Game, vanilla: bool) {
    let game_config = config.game_config(game);

    // Get game path
    let game_path = match &game_config.path {
        Some(p) => p,
        None => {
            eprintln!(
                "No game path configured for {:?}. Use 'config path {:?} --detect' first.",
                game, game
            );
            std::process::exit(1);
        }
    };

    // Get mod path (unless vanilla)
    let mod_path = if vanilla {
        None
    } else {
        game_config.mod_path.as_ref()
    };

    // Launch the game
    match launch::launch_game(game, game_path, mod_path) {
        Ok(()) => {
            if let Some(mp) = mod_path {
                println!("Launched {:?} with mod: {}", game, mp.display());
            } else {
                println!("Launched {:?} (vanilla)", game);
            }
        }
        Err(e) => {
            eprintln!("Failed to launch game: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_config_command(config: &mut config::Config, command: &ConfigCommands) {
    match command {
        ConfigCommands::Show => {
            println!(
                "Config: {}",
                config::Config::config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "unknown".to_string())
            );
            println!();
            println!("[Halo Wars 1]");
            print_game_config(&config.halo_wars_1);
            println!();
            println!("[Halo Wars 2] (not yet supported)");
            print_game_config(&config.halo_wars_2);
        }
        ConfigCommands::Path {
            game,
            detect,
            clear,
            path,
        } => {
            if *clear {
                // Clear path
                let game_config = config.game_config_mut(*game);
                game_config.path = None;
                game_config.auto_detected = false;
                save_config(config);
                println!("Cleared path for {:?}", game);
            } else if *detect {
                // Auto-detect and save
                if let Some(detected) = detection::detect_game(*game) {
                    let game_config = config.game_config_mut(*game);
                    game_config.path = Some(detected.clone());
                    game_config.auto_detected = true;
                    save_config(config);
                    println!("{}", detected.display());
                } else {
                    eprintln!("Could not auto-detect {:?}", game);
                    std::process::exit(1);
                }
            } else if let Some(p) = path {
                // Set path
                if !p.exists() {
                    eprintln!("Warning: Path does not exist: {}", p.display());
                }
                config.set_game_path(*game, p.clone());
                save_config(config);
                println!("{}", p.display());
            } else {
                // Get path
                let game_config = config.game_config(*game);
                match &game_config.path {
                    Some(p) => println!("{}", p.display()),
                    None => {
                        eprintln!("No path configured for {:?}", game);
                        std::process::exit(1);
                    }
                }
            }
        }
        ConfigCommands::ModPath { game, clear, path } => {
            if *clear {
                // Clear mod path
                let game_config = config.game_config_mut(*game);
                game_config.mod_path = None;
                save_config(config);
                println!("Cleared mod path for {:?}", game);
            } else if let Some(p) = path {
                // Set mod path
                if !p.exists() {
                    eprintln!("Warning: Path does not exist: {}", p.display());
                }
                let game_config = config.game_config_mut(*game);
                game_config.mod_path = Some(p.clone());
                save_config(config);
                println!("{}", p.display());
            } else {
                // Get mod path
                let game_config = config.game_config(*game);
                match &game_config.mod_path {
                    Some(p) => println!("{}", p.display()),
                    None => {
                        eprintln!("No mod path configured for {:?}", game);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}

fn save_config(config: &config::Config) {
    if let Err(e) = config.save() {
        eprintln!("Failed to save config: {}", e);
        std::process::exit(1);
    }
}

fn print_game_config(config: &config::GameConfig) {
    match &config.path {
        Some(path) => {
            println!("  Path: {}", path.display());
            println!("  Auto-detected: {}", config.auto_detected);
        }
        None => println!("  Path: not configured"),
    }
    match &config.mod_path {
        Some(path) => println!("  Mod path: {}", path.display()),
        None => println!("  Mod path: not configured"),
    }
}
