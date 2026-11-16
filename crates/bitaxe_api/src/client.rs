use std::time::Duration;

use log::debug;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Body, Client, Method, Response};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::models::{Error, Result, Settings, SystemInfo};
use crate::settings;

pub struct BitaxeClient {
    client: Client,
    base: String,
}

impl BitaxeClient {
    pub fn new(base: impl ToString) -> Result<Self> {
        let base = base.to_string();

        debug!("Initializing Bitaxe client at {base}");
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

        Ok(Self { client, base })
    }

    pub fn new_with_client(client: Client, base: impl ToString) -> Self {
        let base = base.to_string();

        debug!("Initializing Bitaxe client with client at {base}");

        Self {
            client,
            base: base.to_string(),
        }
    }

    pub async fn system_info(&self) -> Result<SystemInfo> {
        let body: Option<()> = None;

        self.send_request(Method::GET, "/system/info", body)
            .await?
            .json::<SystemInfo>()
            .await
            .map_err(Error::from)
    }

    pub async fn restart(&self) -> Result<()> {
        let body: Option<()> = None;
        let response = self
            .send_request(Method::POST, "/system/restart", body)
            .await?
            .text()
            .await?;

        debug!("Restart response: {response}");
        Ok(())
    }

    /// Applies settings using the request format of the device's firmware version.
    ///
    /// This reads the device info first to detect the firmware version. Settings the firmware
    /// cannot apply are rejected with [`Error::UnsupportedSetting`] before anything is sent.
    pub async fn update_settings(&self, settings: Settings) -> Result<()> {
        let info = self.system_info().await?;
        let plan = settings::plan(&settings, &info)?;

        if !plan.patch.is_empty() {
            self.patch_system(plan.patch).await?;
        }

        for (index, pool) in plan.pools {
            self.update_pool(index, pool).await?;
        }

        Ok(())
    }

    /// Sends a raw `PATCH /api/system` body without any version translation.
    pub async fn patch_system(&self, body: Map<String, Value>) -> Result<()> {
        let response = self
            .send_request(Method::PATCH, "/system", Some(body))
            .await?
            .text()
            .await?;

        debug!("Settings response: {response}");
        Ok(())
    }

    /// Replaces a pool slot with `PUT /api/system/pools/{index}`. Requires firmware v2.15.0 or
    /// newer.
    pub async fn update_pool(&self, index: u16, pool: Map<String, Value>) -> Result<()> {
        let response = self
            .send_request(Method::PUT, &format!("/system/pools/{index}"), Some(pool))
            .await?
            .text()
            .await?;

        debug!("Pool {index} response: {response}");
        Ok(())
    }

    fn gen_url(&self, path: &str) -> String {
        format!("http://{}/api{}", self.base, path)
    }

    pub async fn upload_firmware_file(&self, contents: impl Into<Body>) -> Result<()> {
        self.upload_file("/system/OTA", contents).await
    }

    pub async fn upload_www_file(&self, contents: impl Into<Body>) -> Result<()> {
        self.upload_file("/system/OTAWWW", contents).await
    }

    async fn upload_file(&self, path: &str, contents: impl Into<Body>) -> Result<()> {
        debug!("Uploading contents to {path}");
        let response = self
            .client
            .post(self.gen_url(path))
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(contents)
            .send()
            .await?;

        Self::check_status(response).await?;

        Ok(())
    }

    async fn send_request(
        &self,
        method: Method,
        path: &str,
        body: Option<impl Serialize>,
    ) -> Result<Response> {
        let mut request = self.client.request(method.clone(), self.gen_url(path));

        if let Some(body) = body {
            request = request.json(&body);
        }

        debug!("Sending {method} to {path}");
        let response = request.send().await?;
        debug!(
            "Received response from Bitaxe API: {}",
            response.status().as_u16()
        );

        Self::check_status(response).await
    }

    async fn check_status(response: Response) -> Result<Response> {
        let status = response.status();

        if status.is_redirection() {
            // the firmware answers unknown API paths with a redirect instead of a 404
            Err(Error::InvalidRequest(status))
        } else if status.is_client_error() {
            Err(Error::ApiClient(status, response.text().await?))
        } else if status.is_server_error() {
            Err(Error::ApiServer(status, response.text().await?))
        } else {
            Ok(response)
        }
    }
}
