use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_cluster_status(&self) -> Result<Vec<Value>> {
        Ok(self.request(Method::GET, "cluster/status", None).await?)
    }

    pub async fn get_cluster_log(&self, limit: Option<u64>) -> Result<Vec<Value>> {
        let mut path = "cluster/log".to_string();
        if let Some(l) = limit {
            path.push_str(&format!("?max={}", l));
        }
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn get_firewall_rules(
        &self,
        node: Option<&str>,
        vmid: Option<i64>,
    ) -> Result<Vec<Value>> {
        let path = if let (Some(n), Some(id)) = (node, vmid) {
            let (_, vm_type) = self.find_vm_location(id).await?;
            format!("nodes/{}/{}/{}/firewall/rules", n, vm_type, id)
        } else if let Some(n) = node {
            format!("nodes/{}/firewall/rules", n)
        } else {
            "cluster/firewall/rules".to_string()
        };
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn add_firewall_rule(
        &self,
        node: Option<&str>,
        vmid: Option<i64>,
        params: &Value,
    ) -> Result<()> {
        let path = if let (Some(n), Some(id)) = (node, vmid) {
            let (_, vm_type) = self.find_vm_location(id).await?;
            format!("nodes/{}/{}/{}/firewall/rules", n, vm_type, id)
        } else if let Some(n) = node {
            format!("nodes/{}/firewall/rules", n)
        } else {
            "cluster/firewall/rules".to_string()
        };
        Ok(self.request(Method::POST, &path, Some(params)).await?)
    }

    pub async fn delete_firewall_rule(
        &self,
        node: Option<&str>,
        vmid: Option<i64>,
        pos: i64,
    ) -> Result<()> {
        let path = if let (Some(n), Some(id)) = (node, vmid) {
            let (_, vm_type) = self.find_vm_location(id).await?;
            format!("nodes/{}/{}/{}/firewall/rules/{}", n, vm_type, id, pos)
        } else if let Some(n) = node {
            format!("nodes/{}/firewall/rules/{}", n, pos)
        } else {
            format!("cluster/firewall/rules/{}", pos)
        };
        Ok(self.request(Method::DELETE, &path, None).await?)
    }

    pub async fn get_task_status(&self, node: &str, upid: &str) -> Result<Value> {
        let path = format!("nodes/{}/tasks/{}/status", node, upid);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn wait_for_task(&self, node: &str, upid: &str, timeout_secs: u64) -> Result<Value> {
        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);

        loop {
            if start_time.elapsed() > timeout_duration {
                return Err(crate::proxmox::error::ProxmoxError::Timeout(format!(
                    "Timeout waiting for task {}",
                    upid
                ))
                .into());
            }

            let status = self.get_task_status(node, upid).await?;

            if let Some(s) = status.get("status").and_then(|v| v.as_str()) {
                if s == "stopped" {
                    return Ok(status);
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    pub async fn get_task_log(&self, node: &str, upid: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/tasks/{}/log", node, upid);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn list_tasks(&self, node: &str, limit: Option<u64>) -> Result<Vec<Value>> {
        let mut path = format!("nodes/{}/tasks", node);
        if let Some(l) = limit {
            path.push_str(&format!("?limit={}", l));
        }
        Ok(self.request(Method::GET, &path, None).await?)
    }

    // --- Cluster Management ---

    pub async fn create_cluster(&self, clustername: &str) -> Result<String> {
        let params = json!({ "clustername": clustername });
        Ok(self
            .request(Method::POST, "cluster/config", Some(&params))
            .await?)
    }

    pub async fn get_join_info(&self) -> Result<Value> {
        Ok(self
            .request(Method::GET, "cluster/config/join", None)
            .await?)
    }

    pub async fn join_cluster(
        &self,
        hostname: &str,
        password: &str,
        fingerprint: &str,
    ) -> Result<String> {
        let params = json!({
            "hostname": hostname,
            "password": password,
            "fingerprint": fingerprint
        });
        Ok(self
            .request(Method::POST, "cluster/config/join", Some(&params))
            .await?)
    }

    // --- HA Management ---

    pub async fn get_ha_resources(&self) -> Result<Vec<Value>> {
        Ok(self
            .request(Method::GET, "cluster/ha/resources", None)
            .await?)
    }

    pub async fn get_ha_groups(&self) -> Result<Vec<Value>> {
        Ok(self.request(Method::GET, "cluster/ha/groups", None).await?)
    }

    pub async fn add_ha_resource(&self, sid: &str, params: &Value) -> Result<()> {
        let mut full_params = params
            .as_object()
            .ok_or(anyhow::anyhow!("Params must be object"))?
            .clone();
        full_params.insert("sid".to_string(), json!(sid));
        let _: Value = self
            .request(
                Method::POST,
                "cluster/ha/resources",
                Some(&Value::Object(full_params)),
            )
            .await?;
        Ok(())
    }

    pub async fn update_ha_resource(&self, sid: &str, params: &Value) -> Result<()> {
        let path = format!("cluster/ha/resources/{}", sid);
        let _: Value = self.request(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_ha_resource(&self, sid: &str) -> Result<()> {
        let path = format!("cluster/ha/resources/{}", sid);
        let _: Value = self.request(Method::DELETE, &path, None).await?;
        Ok(())
    }
}
