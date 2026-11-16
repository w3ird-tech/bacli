use std::time::Duration;

use anyhow::Result;
use bitaxe_api::client::BitaxeClient;
use bitaxe_api::models::SystemInfo;
use log::debug;

use crate::config::Config;
use crate::models::InfoArgs;

pub async fn get_info(config: Config, args: InfoArgs) -> Result<()> {
    debug!("Getting device info: {args:?}");
    let base = config
        .get_device(&args.base)
        .cloned()
        .map(|b| b.base)
        .unwrap_or(args.base);

    let client = BitaxeClient::new(&base)?;
    let info = client.system_info().await?;
    debug!("Device info: {info:?}");

    let output = if args.json {
        serde_json::to_string(&info)?
    } else {
        build_table(&base, info)
    };

    println!("{output}");

    Ok(())
}

fn build_table(base: &str, info: SystemInfo) -> String {
    let runtime = humantime::format_duration(Duration::from_secs(info.uptime_seconds));
    let unknown = || "Unknown".to_string();
    let fallback = match (&info.fallback_stratum_url, info.fallback_stratum_port) {
        (Some(url), Some(port)) if !url.is_empty() => format!("{url}:{port}"),
        _ => "Not configured".to_string(),
    };

    format!(
        r#"Address: {}
Board: {}
ESP Miner: {}
Uptime: {}

Mining
------
Hash Rate: {} GH/s
Shares: {}

Wifi
----
SSID: {}
Status: {}

Main Pool
---------
URL: {}:{}
User: {}

Fallback Pool
---------
URL: {}
User: {}
"#,
        base,
        info.board_version.unwrap_or_else(unknown),
        info.version,
        runtime,
        info.hash_rate.round(),
        info.shares_accepted,
        info.ssid,
        info.wifi_status,
        info.stratum_url,
        info.stratum_port,
        info.stratum_user,
        fallback,
        info.fallback_stratum_user.unwrap_or_default(),
    )
}
