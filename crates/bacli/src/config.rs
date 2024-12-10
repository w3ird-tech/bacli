use std::{io::ErrorKind, path::PathBuf};

use anyhow::{anyhow, Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::fs::{self, File};

use crate::models::Device;

#[derive(Debug, Clone)]
pub struct Config {
    path: PathBuf,
    inner: AppConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub devices: Vec<Device>,
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
        let path = path.into();
        let cfg = config::Config::builder()
            .add_source(config::File::from(path.as_path()))
            .build()?;

        Ok(Config {
            path,
            inner: cfg.try_deserialize().map_err(Error::from)?,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.inner.devices.is_empty()
    }

    pub fn get_devices(&self) -> &[Device] {
        &self.inner.devices
    }

    pub fn get_device(&self, ident: &str) -> Option<&Device> {
        self.inner.devices.iter().find(|d| d.matches_ident(ident))
    }

    pub fn get_device_mut(&mut self, ident: &str) -> Option<&mut Device> {
        self.inner
            .devices
            .iter_mut()
            .find(|d| d.matches_ident(ident))
    }

    pub async fn upsert_device(
        &mut self,
        base: impl ToString,
        alias: Option<String>,
    ) -> Result<()> {
        let base = base.to_string();

        if let Some(device) = self.get_device_mut(&base) {
            device.alias = alias;
        } else {
            self.inner.devices.push(Device { base, alias });
        }

        self.save().await?;

        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        let content = serde_yaml::to_string(&self.inner)?;
        fs::write(&self.path, content).await?;

        Ok(())
    }
}
