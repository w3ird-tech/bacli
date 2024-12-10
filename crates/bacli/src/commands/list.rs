use anyhow::Result;
use comfy_table::Table;

use crate::config::Config;

pub async fn list(config: Config) -> Result<()> {
    if config.is_empty() {
        println!("No devices currently configured.");
    } else {
        let rows = config
            .get_devices()
            .iter()
            .map(|d| vec![d.base.to_string(), d.alias.clone().unwrap_or_default()]);

        let mut table = Table::new();
        table.set_header(vec!["IP", "Alias"]).add_rows(rows);

        println!("{table}");
    }

    Ok(())
}
