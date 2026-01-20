use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    // --- Ceph Status ---

    pub async fn get_ceph_status(&self, node: &str) -> Result<Value> {
        let path = format!("nodes/{}/ceph/status", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    // --- Ceph Pools ---

    pub async fn get_ceph_pools(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/ceph/pools", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn create_ceph_pool(&self, node: &str, name: &str, params: &Value) -> Result<String> {
        let path = format!("nodes/{}/ceph/pools", node);
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("name".to_string(), serde_json::json!(name));

        let res: String = self
            .request(Method::POST, &path, Some(&serde_json::Value::Object(body)))
            .await?;
        Ok(res)
    }

    pub async fn delete_ceph_pool(
        &self,
        node: &str,
        name: &str,
        remove_storages: bool,
    ) -> Result<String> {
        let path = format!("nodes/{}/ceph/pools/{}", node, name);
        let params = serde_json::json!({
            "remove_storages": if remove_storages { 1 } else { 0 }
        });
        let res: String = self.request(Method::DELETE, &path, Some(&params)).await?;
        Ok(res)
    }

    // --- Ceph OSDs ---

    pub async fn get_ceph_osds(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/ceph/osd", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    // --- Ceph Monitors ---

    pub async fn get_ceph_monitors(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/ceph/mon", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }
}
