use std::net::IpAddr;

use bitaxe_api::models::Settings;
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

/// Bitaxe CLI is a wrapper around the Bitaxe API, enabling the management of a Bitaxe device
/// in an easy to use way.
#[derive(Debug, Clone, Parser)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub config: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
    /// Get general information about the device.
    Info(InfoArgs),
    /// Restart the device.
    Restart(RestartArgs),
    /// Update the settings on the device.
    UpdateSettings(UpdateSetttingsArgs),
    /// List known Bitaxe devices from the config
    List,
    /// Associate an alias with a base (IP)
    Alias(AliasArgs),
    /// Scan the local network for devices
    Scan(ScanArgs),
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

#[derive(Debug, Clone, Args)]
pub struct AliasArgs {
    /// The URL of the device on the local network. This will usually be an IP address.
    pub base: String,
    /// The alias to reference the IP.
    pub alias: String,
}

#[derive(Debug, Clone, Args)]
pub struct ScanArgs {
    /// An IP address in the IP range of the network containing the devices.
    #[arg(long, default_value = "192.168.1.1")]
    pub base: IpAddr,
    /// A mask to apply to the base IP to get the range of available IPs.
    #[arg(long, default_value = "255.255.255.0")]
    pub mask: IpAddr,
    /// Save any new found devices to the config.
    #[arg(short, long = "save")]
    pub should_save: bool,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub base: String,
    pub alias: Option<String>,
}

impl Device {
    pub fn matches_ident(&self, ident: &str) -> bool {
        self.base == ident || self.alias.as_ref().is_some_and(|a| a == ident)
    }
}
