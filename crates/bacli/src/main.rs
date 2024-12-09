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
        Command::Info(args) => get_info(args).await?,
        Command::Restart(args) => restart(args).await?,
        Command::UpdateSettings(args) => update_settings(args).await?,
        Command::List => list(cfg.devices).await?,
    }

    Ok(())
}
