use clap::Parser;

use crate::info::get_info;
use crate::models::{Cli, Command};
use crate::restart::restart;
use crate::update_settings::update_settings;

mod info;
mod models;
mod restart;
mod update_settings;

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
