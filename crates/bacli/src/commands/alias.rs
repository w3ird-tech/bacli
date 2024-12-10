use anyhow::Result;

use crate::config::Config;
use crate::models::AliasArgs;

pub async fn alias(mut config: Config, args: AliasArgs) -> Result<()> {
    config.upsert_device(args.base, Some(args.alias)).await?;

    Ok(())
}
