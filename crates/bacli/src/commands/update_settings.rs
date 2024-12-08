use anyhow::Result;
use bitaxe_api::prelude::*;
use log::debug;

use crate::models::UpdateSetttingsArgs;

pub async fn update_settings(args: UpdateSetttingsArgs) -> Result<()> {
    debug!("Updating device settings: {:?}", args);
    let client = BitaxeClient::new(&args.base)?;
    client.update_settings(args.settings).await?;

    eprintln!("Device settings successfully updated.");

    Ok(())
}
