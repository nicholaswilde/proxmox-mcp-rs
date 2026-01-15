use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_cluster_status(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "cluster/status", None).await
    }

    pub async fn get_cluster_log(&self, limit: Option<u64>) -> Result<Vec<Value>> {
        let mut path = "cluster/log".to_string();
        if let Some(l) = limit {
            path.push_str(&format!("?max={}", l));
        }
        self.request(Method::GET, &path, None).await
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
        self.request(Method::GET, &path, None).await
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
        self.request(Method::POST, &path, Some(params)).await
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
        self.request(Method::DELETE, &path, None).await
    }

    pub async fn get_task_status(&self, node: &str, upid: &str) -> Result<Value> {
        let path = format!("nodes/{}/tasks/{}/status", node, upid);
        self.request(Method::GET, &path, None).await
    }

    pub async fn get_task_log(&self, node: &str, upid: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/tasks/{}/log", node, upid);
        self.request(Method::GET, &path, None).await
    }

    pub async fn list_tasks(&self, node: &str, limit: Option<u64>) -> Result<Vec<Value>> {
        let mut path = format!("nodes/{}/tasks", node);
        if let Some(l) = limit {
            path.push_str(&format!("?limit={}", l));
        }
        self.request(Method::GET, &path, None).await
    }

    // --- HA Management ---

    pub async fn get_ha_resources(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "cluster/ha/resources", None)
            .await
    }

    pub async fn get_ha_groups(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "cluster/ha/groups", None).await
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
