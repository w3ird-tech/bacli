use anyhow::Result;
use bitaxe_api::client::BitaxeClient;
use log::debug;

use crate::config::Config;
use crate::models::RestartArgs;

pub async fn restart(config: Config, args: RestartArgs) -> Result<()> {
    debug!("Restarting device: {:?}", args);
    let base = config
        .get_device(&args.base)
        .cloned()
        .map(|b| b.base)
        .unwrap_or(args.base);

    let client = BitaxeClient::new(&base)?;
    client.restart().await?;

    eprintln!("Device successfully restarted.");

    Ok(())
}
