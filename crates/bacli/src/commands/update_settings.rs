use anyhow::Result;
use bitaxe_api::prelude::*;
use log::debug;

use crate::config::Config;
use crate::models::UpdateSetttingsArgs;

pub async fn update_settings(config: Config, args: UpdateSetttingsArgs) -> Result<()> {
    debug!("Updating device settings: {:?}", args);
    let base = config
        .get_device(&args.base)
        .cloned()
        .map(|b| b.base)
        .unwrap_or(args.base);

    let client = BitaxeClient::new(&base)?;
    client.update_settings(args.settings).await?;

    eprintln!("Device settings successfully updated.");

    Ok(())
}
