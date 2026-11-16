use std::cmp::Ordering;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use bitaxe_api::client::BitaxeClient;
use bitaxe_api::models::{Error, SystemInfo};
use bitaxe_api::version::FirmwareVersion;
use log::debug;
use reqwest::header::{HeaderName, ACCEPT};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::models::UpgradeArgs;

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const FIRMWARE_BIN: &str = "esp-miner.bin";
/// Only published for releases before v2.15.0. Newer firmware embeds the web UI.
const WWW_BIN: &str = "www.bin";
const RESTART_TIMEOUT: Duration = Duration::from_secs(300);

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
    debug!("Device info: board={board_version:?}, firmware_version={version}");

    let release = get_release(&http, args.version.as_deref()).await?;
    debug!("Target esp-miner GitHub release: {}", release.tag_name);
    let target = release.tag_name.as_str();

    match compare_versions(&version, target) {
        Some(Ordering::Equal) if !args.force => {
            eprintln!("Device '{base}' is up-to-date. Device version: {version}");
            return Ok(());
        }
        Some(Ordering::Greater) if !args.force => {
            bail!(
                "Device '{base}' runs {version}, which is newer than {target}. Pass --force to \
                 downgrade."
            );
        }
        _ => {}
    }

    println!(
        "Device '{base}' will be updated. Device version: {version}, Target version: {target}"
    );

    let firmware = release
        .asset(FIRMWARE_BIN)
        .ok_or_else(|| anyhow!("Release {target} has no {FIRMWARE_BIN} to install over the air"))?;
    let www = release.asset(WWW_BIN);

    if !args.execute {
        let www_step = match www {
            Some(_) => "\n4. Upload the www file to /api/system/OTAWWW.",
            None => "",
        };
        let files = match www {
            Some(_) => "firmware and www bins",
            None => "firmware bin (the web UI is embedded in this release)",
        };

        eprintln!(
            r#"This tool will perform the following:

1. Download the {target} {files} and verify their checksums.
2. Upload the firmware file to /api/system/OTA.
3. Wait for the device to restart.{www_step}

Note: This is an experimental command. Use the device's web page if you're unsure.

Pass --execute to run the update.
"#
        );

        return Ok(());
    }

    // Download everything before touching the device so a missing or corrupt file can't leave
    // it half-updated.
    let firmware_file = download_verified(&http, &release, firmware).await?;
    let www_file = match www {
        Some(asset) => Some(download_verified(&http, &release, asset).await?),
        None => None,
    };

    eprintln!("Uploading {FIRMWARE_BIN} to '{base}'...");
    client.upload_firmware_file(firmware_file).await?;

    // The device auto-restarts when new firmware is uploaded. So we will wait for it to come
    // back up before proceeding.
    let mut info = wait_for_restart(&client).await?;

    if let Some(www_file) = www_file {
        eprintln!("Uploading {WWW_BIN} to '{base}'...");
        client.upload_www_file(www_file).await?;
        info = client.system_info().await?;
    }

    if compare_versions(&info.version, target) == Some(Ordering::Equal) {
        eprintln!("Bitaxe {base} successfully updated to {}.", info.version);
    } else {
        eprintln!(
            "Warning: Bitaxe {base} reports version {} after updating to {target}. The device \
             may have rolled back to its previous firmware.",
            info.version
        );
    }

    Ok(())
}

/// Compares firmware versions numerically, falling back to exact string equality for
/// versions that can't be parsed.
fn compare_versions(device: &str, target: &str) -> Option<Ordering> {
    match (
        FirmwareVersion::parse(device),
        FirmwareVersion::parse(target),
    ) {
        (Some(device), Some(target)) => Some(device.cmp(&target)),
        _ => (device == target).then_some(Ordering::Equal),
    }
}

async fn wait_for_restart(client: &BitaxeClient) -> Result<SystemInfo> {
    debug!("Waiting for the Bitaxe to restart");
    let deadline = Instant::now() + RESTART_TIMEOUT;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        match client.system_info().await {
            Ok(info) => return Ok(info),
            Err(Error::InvalidRequest(code)) => bail!(
                "Device refused the request as invalid: status code - {}",
                code
            ),
            Err(err) => debug!("Device has not restarted: {err}"),
        }

        if Instant::now() >= deadline {
            bail!(
                "Device did not come back within {} seconds after the firmware upload",
                RESTART_TIMEOUT.as_secs()
            );
        }
    }
}

const GITHUB_API_VERSION: HeaderName = HeaderName::from_static("x-github-api-version");
const GITHUB_RELEASES_URL: &str = "https://api.github.com/repos/bitaxeorg/ESP-Miner/releases";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

impl Release {
    fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    /// `sha256:<hex>`, provided by GitHub for assets uploaded since mid-2025.
    digest: Option<String>,
}

async fn get_release(http: &Client, tag: Option<&str>) -> Result<Release> {
    let url = match tag {
        Some(tag) => format!("{GITHUB_RELEASES_URL}/tags/{tag}"),
        None => format!("{GITHUB_RELEASES_URL}/latest"),
    };

    let release = http
        .get(&url)
        .header(ACCEPT, GITHUB_ACCEPT)
        .header(GITHUB_API_VERSION, "2022-11-28")
        .send()
        .await?
        .error_for_status()
        .with_context(|| match tag {
            Some(tag) => format!("Unable to find esp-miner release {tag}"),
            None => "Unable to find the latest esp-miner release".to_string(),
        })?
        .json::<Release>()
        .await?;

    debug!("GitHub release response: {release:?}");

    Ok(release)
}

async fn download(http: &Client, asset: &Asset) -> Result<Vec<u8>> {
    debug!("Downloading {}", asset.browser_download_url);
    let bytes = http
        .get(&asset.browser_download_url)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("Unable to download {}", asset.name))?
        .bytes()
        .await?;

    Ok(bytes.to_vec())
}

async fn download_verified(http: &Client, release: &Release, asset: &Asset) -> Result<Vec<u8>> {
    let contents = download(http, asset).await?;

    match expected_sha256(http, release, asset).await? {
        Some(expected) => {
            let actual = sha256_hex(&contents);
            if !actual.eq_ignore_ascii_case(&expected) {
                bail!(
                    "Checksum mismatch for {}: expected {expected}, got {actual}",
                    asset.name
                );
            }
            debug!("Verified {} checksum {actual}", asset.name);
        }
        None => eprintln!(
            "Warning: release {} publishes no checksum for {}; skipping verification",
            release.tag_name, asset.name
        ),
    }

    Ok(contents)
}

/// Prefers GitHub's asset digest and falls back to a published `<name>.sha256` file.
async fn expected_sha256(
    http: &Client,
    release: &Release,
    asset: &Asset,
) -> Result<Option<String>> {
    if let Some(hash) = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
    {
        return Ok(Some(hash.to_string()));
    }

    let Some(checksum) = release.asset(&format!("{}.sha256", asset.name)) else {
        return Ok(None);
    };
    let contents = download(http, checksum).await?;

    parse_checksum_file(&String::from_utf8_lossy(&contents))
        .map(Some)
        .ok_or_else(|| anyhow!("{} is not a valid checksum file", checksum.name))
}

/// Parses `sha256sum` output: `<hex>  <file name>`.
fn parse_checksum_file(contents: &str) -> Option<String> {
    let hash = contents.split_whitespace().next()?;
    (hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())).then(|| hash.to_string())
}

fn sha256_hex(contents: &[u8]) -> String {
    Sha256::digest(contents)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert_eq!(compare_versions("v2.9.0", "v2.15.4"), Some(Ordering::Less));
        assert_eq!(
            compare_versions("v2.15.4", "v2.15.4"),
            Some(Ordering::Equal)
        );
        assert_eq!(
            compare_versions("v2.15.0-41-ge3cfbb3", "v2.14.2"),
            Some(Ordering::Greater)
        );
        assert_eq!(compare_versions("custom", "v2.15.4"), None);
        assert_eq!(compare_versions("custom", "custom"), Some(Ordering::Equal));
    }

    #[test]
    fn parses_sha256sum_output() {
        let hash = "a".repeat(64);
        assert_eq!(
            parse_checksum_file(&format!("{hash}  esp-miner.bin\n")),
            Some(hash)
        );
        assert_eq!(parse_checksum_file("not-a-hash  esp-miner.bin"), None);
        assert_eq!(parse_checksum_file(""), None);
    }

    #[test]
    fn hashes_contents() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn release_assets_are_optional() {
        let release: Release = serde_json::from_value(serde_json::json!({
            "tag_name": "v2.15.4",
            "assets": [{
                "name": "esp-miner.bin",
                "browser_download_url": "https://example.com/esp-miner.bin",
                "digest": null,
            }],
        }))
        .unwrap();

        assert!(release.asset(FIRMWARE_BIN).is_some());
        assert!(release.asset(WWW_BIN).is_none());
    }
}
