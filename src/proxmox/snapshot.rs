use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_snapshots(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
    ) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/{}/{}/snapshot", node, resource_type, vmid);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn create_snapshot(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        snapname: &str,
        description: Option<&str>,
        vmstate: bool,
    ) -> Result<String> {
        let path = format!("nodes/{}/{}/{}/snapshot", node, resource_type, vmid);
        let mut params = json!({ "snapname": snapname });
        if let Some(desc) = description {
            params
                .as_object_mut()
                .unwrap()
                .insert("description".to_string(), json!(desc));
        }
        if vmstate {
            params
                .as_object_mut()
                .unwrap()
                .insert("vmstate".to_string(), json!(1));
        }
        let res: String = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(res)
    }

    pub async fn rollback_snapshot(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        snapname: &str,
    ) -> Result<String> {
        let path = format!(
            "nodes/{}/{}/{}/snapshot/{}/rollback",
            node, resource_type, vmid, snapname
        );
        let res: String = self.request(Method::POST, &path, None).await?;
        Ok(res)
    }

    pub async fn delete_snapshot(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        snapname: &str,
    ) -> Result<String> {
        let path = format!(
            "nodes/{}/{}/{}/snapshot/{}",
            node, resource_type, vmid, snapname
        );
        let res: String = self.request(Method::DELETE, &path, None).await?;
        Ok(res)
    }
}
