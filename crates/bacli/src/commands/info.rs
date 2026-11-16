mod view;

use std::io::IsTerminal;

use anyhow::Result;
use bitaxe_api::client::BitaxeClient;
use clap::ColorChoice;
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

    if args.json {
        println!("{}", serde_json::to_string(&info)?);
    } else {
        print!("{}", view::render(&base, &info, use_color(args.color)));
    }

    Ok(())
}

/// Whether to color the output. `auto` colors only when writing to a terminal that has not
/// opted out through `NO_COLOR` or `TERM=dumb`.
fn use_color(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
            let dumb = std::env::var_os("TERM").is_some_and(|term| term == "dumb");
            std::io::stdout().is_terminal() && !no_color && !dumb
        }
    }
}
