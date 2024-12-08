use log::debug;
use reqwest::{Client, Method, Response};
use serde::Serialize;

use crate::models::{Error, Result, Settings, SystemInfo};

pub struct BitaxeClient {
    client: Client,
    base: String,
}

impl BitaxeClient {
    pub fn new(base: impl ToString) -> Result<Self> {
        let base = base.to_string();

        debug!("Initializing Bitaxe client at {}", base);
        let client = Client::builder().build()?;

        Ok(Self { client, base })
    }

    pub fn new_with_client(client: Client, base: impl ToString) -> Self {
        let base = base.to_string();

        debug!("Initializing Bitaxe client with client at {}", base);

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

        debug!("Restart response: {}", response);
        Ok(())
    }

    pub async fn update_settings(&self, settings: Settings) -> Result<()> {
        let response = self
            .send_request(Method::PATCH, "/system", Some(settings))
            .await?
            .text()
            .await?;

        debug!("Settings response: {}", response);
        Ok(())
    }

    async fn send_request(
        &self,
        method: Method,
        path: &str,
        body: Option<impl Serialize>,
    ) -> Result<Response> {
        let url = format!("http://{}/api{}", self.base, path);
        let mut request = self.client.request(method.clone(), url);

        if let Some(body) = body {
            request = request.json(&body);
        }

        debug!("Sending {} to {}", method, path);
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
