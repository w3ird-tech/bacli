use anyhow::Result;
use comfy_table::Table;

use crate::models::DeviceConfig;

pub async fn list(devices: Vec<DeviceConfig>) -> Result<()> {
    if devices.is_empty() {
        println!("No devices currently configured.");
    } else {
        let rows = devices
            .into_iter()
            .map(|d| vec![d.ip.to_string(), d.alias.unwrap_or_default()]);

        let mut table = Table::new();
        table.set_header(vec!["IP", "Alias"]).add_rows(rows);

        println!("{table}");
    }

    Ok(())
}
