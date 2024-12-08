use clap::Parser;

use crate::commands::*;
use crate::models::{Cli, Command};

mod commands;
mod models;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Command::Info(args) => get_info(args).await?,
        Command::Restart(args) => restart(args).await?,
        Command::UpdateSettings(args) => update_settings(args).await?,
    }

    Ok(())
}
