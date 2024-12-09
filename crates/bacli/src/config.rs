use std::{io::ErrorKind, path::PathBuf};

use anyhow::{anyhow, Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::fs::{self, File};

use crate::models::DeviceConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
}

impl Config {
    pub async fn read() -> Result<Self> {
        let dirs =
            ProjectDirs::from("", "", "bacli").ok_or(anyhow!("Unable to initiate project dirs"))?;
        let config_path = dirs.config_dir().join("config.yaml");

        // ensure config directory and file exist
        fs::create_dir_all(dirs.config_dir()).await?;
        match File::create_new(&config_path).await {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err.into()),
        };

        Self::read_from_path(config_path).await
    }

    pub async fn read_from_path(path: impl Into<PathBuf>) -> Result<Self> {
        let cfg = config::Config::builder()
            .add_source(config::File::from(path.into()))
            .build()?;

        cfg.try_deserialize().map_err(Error::from)
    }
}
