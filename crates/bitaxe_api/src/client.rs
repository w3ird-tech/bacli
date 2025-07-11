use std::time::Duration;

use log::debug;
use reqwest::header::CONTENT_TYPE;
use reqwest::{Body, Client, Method, Response};
use serde::Serialize;

use crate::models::{Error, Result, Settings, SystemInfo};

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

    pub async fn update_settings(&self, settings: Settings) -> Result<()> {
        let response = self
            .send_request(Method::PATCH, "/system", Some(settings))
            .await?
            .text()
            .await?;

        debug!("Settings response: {response}");
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
        self.client
            .post(self.gen_url(path))
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(contents)
            .send()
            .await?
            .error_for_status()?;

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
        let status = response.status();

        debug!("Received response from Bitaxe API: {}", status.as_u16());

        if status.is_redirection() {
            // for now, apparently invalid pages come back as a 302 instead of 404
            Err(Error::InvalidRequest(response.status()))
        } else if status.is_server_error() {
            Err(Error::ApiServer(response.status(), response.text().await?))
        } else {
            Ok(response)
        }
    }
}
