use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    // Note: get_network_interfaces is already in system.rs, used by list_networks

    pub async fn create_network_bridge(
        &self,
        node: &str,
        iface: &str,
        params: &Value,
    ) -> Result<()> {
        let path = format!("nodes/{}/network", node);
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("iface".to_string(), serde_json::json!(iface));
        body.insert("type".to_string(), serde_json::json!("bridge"));

        self.request::<()>(Method::POST, &path, Some(&serde_json::Value::Object(body)))
            .await?;
        Ok(())
    }

    pub async fn create_network_bond(&self, node: &str, iface: &str, params: &Value) -> Result<()> {
        let path = format!("nodes/{}/network", node);
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("iface".to_string(), serde_json::json!(iface));
        body.insert("type".to_string(), serde_json::json!("bond"));

        self.request::<()>(Method::POST, &path, Some(&serde_json::Value::Object(body)))
            .await?;
        Ok(())
    }

    pub async fn update_network_interface(
        &self,
        node: &str,
        iface: &str,
        params: &Value,
    ) -> Result<()> {
        let path = format!("nodes/{}/network/{}", node, iface);
        self.request::<()>(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_network_interface(&self, node: &str, iface: &str) -> Result<()> {
        let path = format!("nodes/{}/network/{}", node, iface);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }

    pub async fn apply_network_config(&self, node: &str) -> Result<String> {
        let path = format!("nodes/{}/network", node);
        let res: String = self.request(Method::PUT, &path, None).await?;
        Ok(res)
    }

    pub async fn revert_network_config(&self, node: &str) -> Result<()> {
        let path = format!("nodes/{}/network", node);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }
}
