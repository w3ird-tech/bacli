mod commands;
mod config;
mod models;

use clap::Parser;

use crate::commands::*;
use crate::config::Config;
use crate::models::{Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let cfg = match cli.config {
        Some(path) => Config::read_from_path(path).await,
        None => Config::read().await,
    }?;

    match cli.command {
        Command::Info(args) => get_info(cfg, args).await?,
        Command::Restart(args) => restart(cfg, args).await?,
        Command::UpdateSettings(args) => update_settings(cfg, args).await?,
        Command::List => list(cfg).await?,
        Command::Alias(args) => alias(cfg, args).await?,
        Command::Scan(args) => scan(cfg, args).await?,
        Command::Upgrade(args) => upgrade(cfg, args).await?,
    }

    Ok(())
}
