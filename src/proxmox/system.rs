use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    // --- Network Management ---

    pub async fn get_network_interfaces(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/network", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn get_node_stats(
        &self,
        node: &str,
        timeframe: Option<&str>,
        cf: Option<&str>,
    ) -> Result<Vec<Value>> {
        let mut path = format!("nodes/{}/rrddata", node);
        let mut query = Vec::new();
        if let Some(tf) = timeframe {
            query.push(format!("timeframe={}", tf));
        }
        if let Some(c) = cf {
            query.push(format!("cf={}", c));
        }
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn get_resource_stats(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        timeframe: Option<&str>,
        cf: Option<&str>,
    ) -> Result<Vec<Value>> {
        let mut path = format!("nodes/{}/{}/{}/rrddata", node, resource_type, vmid);
        let mut query = Vec::new();
        if let Some(tf) = timeframe {
            query.push(format!("timeframe={}", tf));
        }
        if let Some(c) = cf {
            query.push(format!("cf={}", c));
        }
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }
        Ok(self.request(Method::GET, &path, None).await?)
    }

    // --- APT Management ---

    pub async fn get_apt_updates(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/apt/update", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn run_apt_update(&self, node: &str) -> Result<String> {
        let path = format!("nodes/{}/apt/update", node);
        Ok(self.request(Method::POST, &path, None).await?)
    }

    pub async fn get_apt_versions(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/apt/versions", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    // --- Service Management ---

    pub async fn get_services(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/services", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn manage_service(&self, node: &str, service: &str, action: &str) -> Result<String> {
        let path = format!("nodes/{}/services/{}/{}", node, service, action);
        Ok(self.request(Method::POST, &path, None).await?)
    }
}
