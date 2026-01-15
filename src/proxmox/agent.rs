use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn agent_ping(&self, node: &str, vmid: i64) -> Result<()> {
        let path = format!("nodes/{}/qemu/{}/agent/ping", node, vmid);
        let _: Value = self.request(Method::POST, &path, None).await?;
        Ok(())
    }

    pub async fn agent_exec(
        &self,
        node: &str,
        vmid: i64,
        command: &[String],
        input_data: Option<&str>,
    ) -> Result<Value> {
        let path = format!("nodes/{}/qemu/{}/agent/exec", node, vmid);
        let mut params = json!({ "command": command });
        if let Some(data) = input_data {
            params
                .as_object_mut()
                .unwrap()
                .insert("input-data".to_string(), json!(data));
        }
        self.request(Method::POST, &path, Some(&params)).await
    }

    pub async fn agent_exec_status(&self, node: &str, vmid: i64, pid: i64) -> Result<Value> {
        let path = format!("nodes/{}/qemu/{}/agent/exec-status?pid={}", node, vmid, pid);
        self.request(Method::GET, &path, None).await
    }

    pub async fn agent_file_read(&self, node: &str, vmid: i64, file: &str) -> Result<Value> {
        let path = format!("nodes/{}/qemu/{}/agent/file-read?file={}", node, vmid, file);
        self.request(Method::GET, &path, None).await
    }

    pub async fn agent_file_write(
        &self,
        node: &str,
        vmid: i64,
        file: &str,
        content: &str,
        encode: Option<bool>,
    ) -> Result<()> {
        let path = format!("nodes/{}/qemu/{}/agent/file-write", node, vmid);
        let mut params = json!({
            "file": file,
            "content": content
        });
        if let Some(enc) = encode {
            params
                .as_object_mut()
                .unwrap()
                .insert("encode".to_string(), json!(if enc { 1 } else { 0 }));
        }
        let _: Value = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(())
    }
}
