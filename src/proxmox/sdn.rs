use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    // --- SDN Zones ---

    pub async fn get_sdn_zones(&self) -> Result<Vec<Value>> {
        let path = "cluster/sdn/zones";
        Ok(self.request(Method::GET, path, None).await?)
    }

    pub async fn create_sdn_zone(&self, zone: &str, zone_type: &str, params: &Value) -> Result<()> {
        let path = "cluster/sdn/zones";
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("zone".to_string(), serde_json::json!(zone));
        body.insert("type".to_string(), serde_json::json!(zone_type));

        self.request::<()>(Method::POST, path, Some(&serde_json::Value::Object(body)))
            .await?;
        Ok(())
    }

    pub async fn delete_sdn_zone(&self, zone: &str) -> Result<()> {
        let path = format!("cluster/sdn/zones/{}", zone);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }

    // --- SDN Vnets ---

    pub async fn get_sdn_vnets(&self) -> Result<Vec<Value>> {
        let path = "cluster/sdn/vnets";
        Ok(self.request(Method::GET, path, None).await?)
    }

    pub async fn create_sdn_vnet(&self, vnet: &str, zone: &str, params: &Value) -> Result<()> {
        let path = "cluster/sdn/vnets";
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("vnet".to_string(), serde_json::json!(vnet));
        body.insert("zone".to_string(), serde_json::json!(zone));

        self.request::<()>(Method::POST, path, Some(&serde_json::Value::Object(body)))
            .await?;
        Ok(())
    }

    pub async fn delete_sdn_vnet(&self, vnet: &str) -> Result<()> {
        let path = format!("cluster/sdn/vnets/{}", vnet);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }

    // --- SDN Apply ---

    pub async fn apply_sdn(&self) -> Result<String> {
        let path = "cluster/sdn";
        let res: String = self.request(Method::PUT, path, None).await?;
        Ok(res)
    }
}
