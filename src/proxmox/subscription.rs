use super::client::ProxmoxClient;
use crate::proxmox::error::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_subscription(&self, node: &str) -> Result<Value> {
        let path = format!("nodes/{}/subscription", node);
        self.request(Method::GET, &path, None).await
    }

    pub async fn set_subscription(&self, node: &str, key: &str) -> Result<()> {
        let path = format!("nodes/{}/subscription", node);
        let params = json!({ "key": key });
        // The API returns void/null on success? Docs say 200 OK.
        // We'll treat it as standard request returning Value but ignore it.
        let _: Value = self.request(Method::PUT, &path, Some(&params)).await?;
        Ok(())
    }

    pub async fn update_subscription(&self, node: &str) -> Result<()> {
        let path = format!("nodes/{}/subscription", node);
        // Force update usually via PUT
        let _: Value = self.request(Method::POST, &path, None).await?;
        Ok(())
    }
}
