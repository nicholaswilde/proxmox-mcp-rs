use crate::proxmox::error::{ProxmoxError, Result as PveResult};
use anyhow::{Context, Result};
use log::info;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

#[derive(Clone)]
pub struct ProxmoxClient {
    pub(crate) client: Client,
    pub(crate) base_url: Url,
    ticket: Option<String>,
    csrf_token: Option<String>,
    api_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TicketResponse {
    data: TicketData,
}

#[derive(Deserialize, Debug)]
struct TicketData {
    ticket: String,
    #[serde(rename = "CSRFPreventionToken")]
    csrf_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VmInfo {
    pub vmid: i64,
    pub name: Option<String>,
    pub status: String,
    pub node: Option<String>,
    #[serde(rename = "type")]
    pub vm_type: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ClusterResource {
    pub vmid: Option<i64>,
    pub node: String,
    #[serde(rename = "type")]
    pub res_type: String,
    pub status: Option<String>,
    pub name: Option<String>,
}

impl ProxmoxClient {
    pub fn new(host: &str, port: u16, verify_ssl: bool) -> Result<Self> {
        let scheme = if host.starts_with("http://") {
            "http"
        } else {
            "https"
        };

        let host_cleaned = if let Some(stripped) = host.strip_prefix("http://") {
            stripped
        } else if let Some(stripped) = host.strip_prefix("https://") {
            stripped
        } else {
            host
        };
        let host_cleaned = host_cleaned.trim_end_matches('/');

        let url_str = format!("{}://{}:{}/api2/json/", scheme, host_cleaned, port);

        let base_url = Url::parse(&url_str).context("Invalid host URL")?;

        let client = Client::builder()
            .danger_accept_invalid_certs(!verify_ssl)
            .cookie_store(true)
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self {
            client,
            base_url,
            ticket: None,
            csrf_token: None,
            api_token: None,
        })
    }

    pub fn set_api_token(&mut self, user: &str, token_name: &str, token_value: &str) {
        self.api_token = Some(format!(
            "PVEAPIToken={}!{}={}",
            user, token_name, token_value
        ));
    }

    pub async fn login(&mut self, user: &str, password: &str) -> Result<()> {
        let url = self.base_url.join("access/ticket")?;
        let params = [("username", user), ("password", password)];

        let resp = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await
            .context("Login request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProxmoxError::Auth(format!("{} - {}", status, text)).into());
        }

        let body: TicketResponse = resp
            .json()
            .await
            .context("Failed to parse login response")?;

        self.ticket = Some(body.data.ticket);
        self.csrf_token = Some(body.data.csrf_token);

        info!("Successfully logged in as {}", user);
        Ok(())
    }

    pub(crate) async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> PveResult<T> {
        let url = self.base_url.join(path).map_err(ProxmoxError::Url)?;
        let mut req = self.client.request(method, url);

        if let Some(token) = &self.api_token {
            req = req.header("Authorization", token);
        } else {
            if let Some(token) = &self.csrf_token {
                req = req.header("CSRFPreventionToken", token);
            }
            if let Some(ticket) = &self.ticket {
                req = req.header("Cookie", format!("PVEAuthCookie={}", ticket));
            }
        }

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req.send().await.map_err(ProxmoxError::Request)?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProxmoxError::Api(status, text));
        }

        let v: Value = resp.json().await.map_err(ProxmoxError::Request)?;
        if let Some(data) = v.get("data") {
            serde_json::from_value(data.clone()).map_err(ProxmoxError::Json)
        } else {
            serde_json::from_value(v).map_err(ProxmoxError::Json)
        }
    }
}
