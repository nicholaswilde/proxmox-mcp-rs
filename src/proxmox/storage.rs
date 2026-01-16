use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_storage_list(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/storage", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn get_cluster_storage(&self) -> Result<Vec<Value>> {
        Ok(self.request(Method::GET, "storage", None).await?)
    }

    pub async fn get_storage_content(
        &self,
        node: &str,
        storage: &str,
        content_type: Option<&str>,
    ) -> Result<Vec<Value>> {
        let mut path = format!("nodes/{}/storage/{}/content", node, storage);
        if let Some(ct) = content_type {
            path.push_str(&format!("?content={}", ct));
        }
        Ok(self.request(Method::GET, &path, None).await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_storage(
        &self,
        storage: &str,
        storage_type: &str,
        content: Option<&str>,
        nodes: Option<Vec<String>>,
        enable: Option<bool>,
        extra_params: Option<&serde_json::Map<String, Value>>,
    ) -> Result<()> {
        let mut params = json!({
            "storage": storage,
            "type": storage_type,
        });

        if let Some(c) = content {
            params
                .as_object_mut()
                .unwrap()
                .insert("content".to_string(), json!(c));
        }
        if let Some(n) = nodes {
            params
                .as_object_mut()
                .unwrap()
                .insert("nodes".to_string(), json!(n.join(",")));
        }
        if let Some(e) = enable {
            params
                .as_object_mut()
                .unwrap()
                .insert("disable".to_string(), json!(if e { 0 } else { 1 }));
        }
        if let Some(extra) = extra_params {
            for (k, v) in extra {
                params.as_object_mut().unwrap().insert(k.clone(), v.clone());
            }
        }
        let _: Value = self.request(Method::POST, "storage", Some(&params)).await?;
        Ok(())
    }

    pub async fn delete_storage(&self, storage: &str) -> Result<()> {
        let path = format!("storage/{}", storage);
        let _: Value = self.request(Method::DELETE, &path, None).await?;
        Ok(())
    }

    pub async fn update_storage(
        &self,
        storage: &str,
        params: &serde_json::Map<String, Value>,
    ) -> Result<()> {
        let path = format!("storage/{}", storage);
        let _: Value = self
            .request(Method::PUT, &path, Some(&Value::Object(params.clone())))
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn download_url(
        &self,
        node: &str,
        storage: &str,
        url: &str,
        filename: &str,
        content: &str,
        checksum: Option<&str>,
        checksum_algorithm: Option<&str>,
    ) -> Result<String> {
        let path = format!("nodes/{}/storage/{}/download-url", node, storage);
        let mut params = json!({
            "url": url,
            "filename": filename,
            "content": content,
        });
        if let Some(cs) = checksum {
            params
                .as_object_mut()
                .unwrap()
                .insert("checksum".to_string(), json!(cs));
        }
        if let Some(algo) = checksum_algorithm {
            params
                .as_object_mut()
                .unwrap()
                .insert("checksum-algorithm".to_string(), json!(algo));
        }
        let res: String = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(res)
    }

    // --- Backup Management ---

    pub async fn get_backups(
        &self,
        node: &str,
        storage: &str,
        vmid: Option<i64>,
    ) -> Result<Vec<Value>> {
        let backups = self
            .get_storage_content(node, storage, Some("backup"))
            .await?;
        if let Some(id) = vmid {
            let filtered = backups
                .into_iter()
                .filter(|b| {
                    if let Some(bid) = b.get("vmid").and_then(|v| v.as_i64()) {
                        return bid == id;
                    }
                    if let Some(volid) = b.get("volid").and_then(|v| v.as_str()) {
                        return volid.contains(&format!("-{}", id));
                    }
                    false
                })
                .collect();
            Ok(filtered)
        } else {
            Ok(backups)
        }
    }

    pub async fn create_backup(
        &self,
        node: &str,
        vmid: i64,
        storage: Option<&str>,
        mode: Option<&str>,
        compress: Option<&str>,
        remove: Option<bool>,
    ) -> Result<String> {
        let path = format!("nodes/{}/vzdump", node);
        let mut params = json!({ "vmid": vmid });
        if let Some(s) = storage {
            params
                .as_object_mut()
                .unwrap()
                .insert("storage".to_string(), json!(s));
        }
        if let Some(m) = mode {
            params
                .as_object_mut()
                .unwrap()
                .insert("mode".to_string(), json!(m));
        }
        if let Some(c) = compress {
            params
                .as_object_mut()
                .unwrap()
                .insert("compress".to_string(), json!(c));
        }
        if let Some(r) = remove {
            params
                .as_object_mut()
                .unwrap()
                .insert("remove".to_string(), json!(if r { 1 } else { 0 }));
        }
        let res: String = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(res)
    }

    pub async fn restore_backup(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        archive: &str,
        storage: Option<&str>,
        force: Option<bool>,
    ) -> Result<String> {
        let path = format!("nodes/{}/{}", node, resource_type);
        let mut params = json!({
            "vmid": vmid,
            "archive": archive,
            "restore": 1
        });

        if resource_type == "lxc" {
            params.as_object_mut().unwrap().remove("archive");
            params
                .as_object_mut()
                .unwrap()
                .insert("ostemplate".to_string(), json!(archive));
        }

        if let Some(s) = storage {
            params
                .as_object_mut()
                .unwrap()
                .insert("storage".to_string(), json!(s));
        }
        if let Some(f) = force {
            params
                .as_object_mut()
                .unwrap()
                .insert("force".to_string(), json!(if f { 1 } else { 0 }));
        }

        let res: String = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(res)
    }
}
