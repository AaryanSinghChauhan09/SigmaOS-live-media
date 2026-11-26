//! CLI for sigmaos-live-media.

use clap::{Parser, Subcommand};
use sigmaos_live_media::{default_config_dir, default_config_path, LiveMediaConfig};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sigmaos-live-media", about = "SigmaOS live wallpaper manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a live-media.toml file.
    Validate {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// List known wallpapers.
    List {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print default config as TOML.
    Default,
    /// Apply config to user config dir.
    Apply {
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

fn load(file: Option<PathBuf>) -> Result<LiveMediaConfig, Box<dyn std::error::Error>> {
    match file.or_else(default_config_path) {
        Some(p) => Ok(LiveMediaConfig::load(&p)?),
        None => Err("cannot determine config path".into()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    match Cli::parse().command {
        Commands::Validate { file } => {
            let cfg = load(file)?;
            let w = cfg.active_wallpaper().unwrap();
            println!(
                "valid: active='{}' kind={:?} path={}",
                cfg.active,
                w.kind,
                cfg.resolve_path(w).display()
            );
        }
        Commands::List { file } => {
            let cfg = load(file)?;
            for w in &cfg.wallpapers {
                let marker = if w.name == cfg.active { "*" } else { " " };
                println!("{} {} ({:?})", marker, w.name, w.kind);
            }
        }
        Commands::Default => {
            print!("{}", LiveMediaConfig::default().to_toml()?);
        }
        Commands::Apply { file } => {
            let cfg = match file {
                Some(p) => LiveMediaConfig::load(&p)?,
                None => LiveMediaConfig::default(),
            };
            let dir = default_config_dir().ok_or("cannot determine config dir")?;
            println!("applied: {}", cfg.apply(&dir)?.display());
        }
    }
    Ok(())
}
