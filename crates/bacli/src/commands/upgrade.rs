use std::time::Duration;

use anyhow::{bail, Result};
use bitaxe_api::client::BitaxeClient;
use bitaxe_api::models::{Error, SystemInfo};
use log::debug;
use reqwest::header::{HeaderName, ACCEPT};
use reqwest::{Client, Response};
use serde::Deserialize;

use crate::config::Config;
use crate::models::UpgradeArgs;

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const FIRMWARE_BIN: &str = "esp-miner.bin";
const WWW_BIN: &str = "www.bin";

pub async fn upgrade(config: Config, args: UpgradeArgs) -> Result<()> {
    let base = config
        .get_device(&args.base)
        .cloned()
        .map(|b| b.base)
        .unwrap_or(args.base);

    let http = Client::builder().user_agent(APP_USER_AGENT).build()?;
    let client = BitaxeClient::new_with_client(http.clone(), &base);
    let SystemInfo {
        board_version,
        version,
        ..
    } = client.system_info().await?;
    debug!("Device info: board={board_version}, firmware_version={version}");

    let latest_release = get_latest_release(&http).await?;
    debug!("Latest esp-miner GitHub release: {latest_release}");

    if version == latest_release && !args.force {
        eprintln!("Device '{base}' is up-to-date. Device version: {version}");
        return Ok(());
    }

    println!(
        "Device '{base}' is out-of-date. Device version: {version}, Latest version: {latest_release}"
    );

    if !args.execute {
        eprintln!(
            r#"This tool will perform the following:

1. Download the most recent ({latest_release}) firmware and www bins.
2. Upload the firmware file to /api/system/OTA.
3. Upload the www file to /api/system/OTAWWW.

Note: This is an experimental command. Use the device's web page if you're unsure.

Pass --execute to run the update.
"#
        );

        return Ok(());
    }

    let firmware_file = download_file(&http, &latest_release, FIRMWARE_BIN)
        .await?
        .bytes()
        .await?;
    client.upload_firmware_file(firmware_file).await?;

    // The device auto-restarts when new firmware/www is uploaded. So we will wait for it to come
    // back up before proceeding.
    wait_for_restart(&client).await?;

    let www_file = download_file(&http, &latest_release, WWW_BIN)
        .await?
        .bytes()
        .await?;
    client.upload_www_file(www_file).await?;

    eprintln!("Bitaxe {base} successfully updated.");

    Ok(())
}

async fn wait_for_restart(client: &BitaxeClient) -> Result<()> {
    debug!("Waiting for the Bitaxe to restart");

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        match client.system_info().await {
            Ok(_) => return Ok(()),
            Err(Error::InvalidRequest(code)) => bail!(
                "Device refused the request as invalid: status code - {}",
                code
            ),
            Err(Error::Http(_)) | Err(Error::ApiServer(_, _)) => {}
        }

        debug!("Device has not restarted. Continuing to wait.");
    }
}

const GITHUB_API_VERSION: HeaderName = HeaderName::from_static("x-github-api-version");
const GITHUB_LATEST_URL: &str = "https://api.github.com/repos/skot/esp-miner/releases/latest";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
}

async fn get_latest_release(http: &Client) -> Result<String> {
    let response = http
        .get(GITHUB_LATEST_URL)
        .header(ACCEPT, GITHUB_ACCEPT)
        .header(GITHUB_API_VERSION, "2022-11-28")
        .send()
        .await?
        .error_for_status()?
        .json::<ReleaseResponse>()
        .await?;

    debug!("GitHub latest release response: {response:?}");

    Ok(response.tag_name)
}

async fn download_file(http: &Client, version: &str, filename: &str) -> Result<Response> {
    let url = format!("https://github.com/skot/ESP-Miner/releases/download/{version}/{filename}");
    let response = http.get(url).send().await?.error_for_status()?;

    Ok(response)
}
