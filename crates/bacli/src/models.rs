use bitaxe_api::models::Settings;
use clap::{Args, Parser, Subcommand};

/// Bitaxe CLI is a wrapper around the Bitaxe API, enabling the management of a Bitaxe device
/// in an easy to use way.
#[derive(Debug, Clone, Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Get general information about the device.
    Info(InfoArgs),
    /// Restart the device.
    Restart(RestartArgs),
    /// Update the settings on the device.
    UpdateSettings(UpdateSetttingsArgs),
}

#[derive(Debug, Clone, Args)]
pub struct InfoArgs {
    /// The URL of the device on the local network. This will usually be an IP address.
    pub base: String,
    /// Output JSON instead of the formatted information.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct RestartArgs {
    /// The URL of the device on the local network. This will usually be an IP address.
    pub base: String,
}

#[derive(Debug, Clone, Args)]
pub struct UpdateSetttingsArgs {
    /// The URL of the device on the local network. This will usually be an IP address.
    pub base: String,
    #[command(flatten)]
    pub settings: Settings,
}
