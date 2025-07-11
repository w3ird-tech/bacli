use std::net::IpAddr;
use std::time::Duration;

use anyhow::Result;
use bitaxe_api::prelude::*;
use comfy_table::Table;
use futures::future;
use ipnetwork::IpNetwork;
use log::debug;

use crate::config::Config;
use crate::models::ScanArgs;

pub async fn scan(mut config: Config, args: ScanArgs) -> Result<()> {
    debug!("Scanning network for devices: {args:?}");
    let pool = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()?;

    let network = IpNetwork::with_netmask(args.base, args.mask)?;
    let devices = future::join_all(network.into_iter().map(|i| check_ip(pool.clone(), i)))
        .await
        .into_iter()
        .filter_map(|res| res.ok());

    let mut table = Table::new();
    table.set_header(vec!["IP", "Alias", "Board Version", "OS Version"]);

    for (ip, info) in devices {
        let alias = match config.clone().get_device(&ip.to_string()) {
            Some(device) => device.alias.clone().unwrap_or("None".to_string()),
            None => {
                if args.should_save {
                    config.upsert_device(&ip, None).await?;
                }
                "None".to_string()
            }
        };

        table.add_row(vec![
            ip.to_string(),
            alias,
            info.board_version,
            info.version,
        ]);
    }

    println!("{table}");

    Ok(())
}

async fn check_ip(pool: reqwest::Client, ip: IpAddr) -> Result<(IpAddr, SystemInfo)> {
    let client = BitaxeClient::new_with_client(pool, ip);
    let info = client.system_info().await?;

    Ok((ip, info))
}
