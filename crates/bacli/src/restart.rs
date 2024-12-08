use anyhow::Result;
use bitaxe_api::client::BitaxeClient;
use log::debug;

use crate::models::RestartArgs;

pub async fn restart(args: RestartArgs) -> Result<()> {
    debug!("Restarting device: {:?}", args);
    let client = BitaxeClient::new(&args.base)?;
    client.restart().await?;

    eprintln!("Device successfully restarted.");

    Ok(())
}
