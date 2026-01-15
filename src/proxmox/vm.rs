use super::client::{ClusterResource, ProxmoxClient, VmInfo};
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_nodes(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "nodes", None).await
    }

    pub async fn get_all_vms(&self) -> Result<Vec<VmInfo>> {
        let resources = self.get_resources().await?;
        let vms = resources
            .into_iter()
            .filter(|r| (r.res_type == "qemu" || r.res_type == "lxc") && r.vmid.is_some())
            .map(|r| VmInfo {
                vmid: r.vmid.unwrap(),
                name: r.name,
                status: r.status.unwrap_or("unknown".to_string()),
                node: Some(r.node),
                vm_type: Some(r.res_type),
            })
            .collect();
        Ok(vms)
    }

    pub async fn get_resources(&self) -> Result<Vec<ClusterResource>> {
        self.request(Method::GET, "cluster/resources", None).await
    }

    pub async fn find_vm_location(&self, vmid: i64) -> Result<(String, String)> {
        let resources = self.get_resources().await?;
        for res in resources {
            if let Some(id) = res.vmid {
                if id == vmid {
                    return Ok((res.node, res.res_type));
                }
            }
        }
        anyhow::bail!("VMID {} not found", vmid);
    }

    pub fn get_console_url(
        &self,
        node: &str,
        vmid: i64,
        vm_type: &str,
        console_type: Option<&str>,
    ) -> Result<String> {
        let mut url = self.base_url.clone();
        url.set_path("/");

        let c_val = if vm_type == "lxc" { "lxc" } else { "kvm" };
        let c_type = console_type.unwrap_or("novnc");

        url.query_pairs_mut()
            .append_pair("console", c_val)
            .append_pair(c_type, "1")
            .append_pair("vmid", &vmid.to_string())
            .append_pair("node", node);

        Ok(url.to_string())
    }

    pub async fn vm_action(
        &self,
        node: &str,
        vmid: i64,
        action: &str,
        vm_type: Option<&str>,
    ) -> Result<String> {
        let type_path = vm_type.unwrap_or("qemu");
        let path = format!("nodes/{}/{}/{}/status/{}", node, type_path, vmid, action);
        let res: String = self.request(Method::POST, &path, None).await?;
        Ok(res)
    }

    pub async fn create_resource(
        &self,
        node: &str,
        resource_type: &str,
        params: &Value,
    ) -> Result<String> {
        let path = format!("nodes/{}/{}", node, resource_type);
        let res: String = self.request(Method::POST, &path, Some(params)).await?;
        Ok(res)
    }

    pub async fn delete_resource(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
    ) -> Result<String> {
        let path = format!("nodes/{}/{}/{}", node, resource_type, vmid);
        let res: String = self.request(Method::DELETE, &path, None).await?;
        Ok(res)
    }

    pub async fn update_config(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        params: &Value,
    ) -> Result<()> {
        let path = format!("nodes/{}/{}/{}/config", node, resource_type, vmid);
        self.request(Method::PUT, &path, Some(params)).await
    }

    pub async fn get_vm_config(&self, node: &str, vmid: i64, resource_type: &str) -> Result<Value> {
        let path = format!("nodes/{}/{}/{}/config", node, resource_type, vmid);
        self.request(Method::GET, &path, None).await
    }

    pub async fn resize_disk(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        disk: &str,
        size: &str,
    ) -> Result<String> {
        let path = format!("nodes/{}/{}/{}/resize", node, resource_type, vmid);
        let params = json!({ "disk": disk, "size": size });
        let res: String = self.request(Method::PUT, &path, Some(&params)).await?;
        Ok(res)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_virtual_disk(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        device: &str,
        storage: &str,
        size_gb: u64,
        format: Option<&str>,
        extra_options: Option<&str>,
    ) -> Result<()> {
        let mut value = format!("{}:{}", storage, size_gb);
        if let Some(fmt) = format {
            value.push_str(&format!(",format={}", fmt));
        }
        if let Some(opts) = extra_options {
            value.push_str(&format!(",{}", opts));
        }
        let params = json!({ device: value });
        self.update_config(node, vmid, resource_type, &params).await
    }

    pub async fn remove_virtual_disk(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        device: &str,
    ) -> Result<()> {
        let params = json!({ "delete": device });
        self.update_config(node, vmid, resource_type, &params).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_network_interface(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        device: &str,
        model: Option<&str>,
        bridge: &str,
        mac: Option<&str>,
        extra_options: Option<&str>,
    ) -> Result<()> {
        let mut value = String::new();
        if resource_type == "qemu" {
            let m = model.unwrap_or("virtio");
            if let Some(addr) = mac {
                value.push_str(&format!("{}={},bridge={}", m, addr, bridge));
            } else {
                value.push_str(&format!("{},bridge={}", m, bridge));
            }
        } else {
            value.push_str(&format!(
                "name=eth{},bridge={}",
                device.replace("net", ""),
                bridge
            ));
            if let Some(addr) = mac {
                value.push_str(&format!(",hwaddr={}", addr));
            }
            if let Some(m) = model {
                if m != "virtio" {
                    value.push_str(&format!(",type={}", m));
                }
            }
        }
        if let Some(opts) = extra_options {
            value.push_str(&format!(",{}", opts));
        }
        let params = json!({ device: value });
        self.update_config(node, vmid, resource_type, &params).await
    }

    pub async fn remove_network_interface(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        device: &str,
    ) -> Result<()> {
        let params = json!({ "delete": device });
        self.update_config(node, vmid, resource_type, &params).await
    }

    // --- Cloud-Init & Configuration ---

    pub async fn set_vm_cloudinit(&self, node: &str, vmid: i64, params: &Value) -> Result<()> {
        let path = format!("nodes/{}/qemu/{}/config", node, vmid);
        self.request(Method::PUT, &path, Some(params)).await
    }

    // --- Resource Tagging ---

    pub async fn add_tag(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        tags: &str,
    ) -> Result<()> {
        let path = format!("nodes/{}/{}/{}/config", node, resource_type, vmid);
        let config = self.get_vm_config(node, vmid, resource_type).await?;
        let current_tags = config.get("tags").and_then(|v| v.as_str()).unwrap_or("");

        let new_tags = if current_tags.is_empty() {
            tags.to_string()
        } else {
            let mut tag_list: Vec<&str> = current_tags.split(&[',', ';', ' '][..]).collect();
            for t in tags.split(&[',', ';', ' '][..]) {
                if !tag_list.contains(&t) {
                    tag_list.push(t);
                }
            }
            tag_list.join(",")
        };

        let params = json!({ "tags": new_tags });
        self.request(Method::PUT, &path, Some(&params)).await
    }

    pub async fn remove_tag(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        tags: &str,
    ) -> Result<()> {
        let path = format!("nodes/{}/{}/{}/config", node, resource_type, vmid);
        let config = self.get_vm_config(node, vmid, resource_type).await?;
        let current_tags = config.get("tags").and_then(|v| v.as_str()).unwrap_or("");

        if current_tags.is_empty() {
            return Ok(());
        }

        let tags_to_remove: Vec<&str> = tags.split(&[',', ';', ' '][..]).collect();
        let new_tag_list: Vec<&str> = current_tags
            .split(&[',', ';', ' '][..])
            .filter(|t| !tags_to_remove.contains(t))
            .collect();

        let new_tags = new_tag_list.join(",");
        let params = json!({ "tags": new_tags });
        self.request(Method::PUT, &path, Some(&params)).await
    }

    pub async fn set_tags(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        tags: &str,
    ) -> Result<()> {
        let path = format!("nodes/{}/{}/{}/config", node, resource_type, vmid);
        let params = json!({ "tags": tags });
        self.request(Method::PUT, &path, Some(&params)).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn clone_resource(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        newid: i64,
        name: Option<&str>,
        target_node: Option<&str>,
        full: Option<bool>,
    ) -> Result<String> {
        let path = format!("nodes/{}/{}/{}/clone", node, resource_type, vmid);
        let mut params = json!({ "newid": newid });
        if let Some(n) = name {
            params
                .as_object_mut()
                .unwrap()
                .insert("name".to_string(), json!(n));
        }
        if let Some(t) = target_node {
            params
                .as_object_mut()
                .unwrap()
                .insert("target".to_string(), json!(t));
        }
        if let Some(f) = full {
            params
                .as_object_mut()
                .unwrap()
                .insert("full".to_string(), json!(if f { 1 } else { 0 }));
        }
        let res: String = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(res)
    }

    pub async fn migrate_resource(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        target_node: &str,
        online: bool,
    ) -> Result<String> {
        let path = format!("nodes/{}/{}/{}/migrate", node, resource_type, vmid);
        let mut params = json!({ "target": target_node });
        if online {
            params
                .as_object_mut()
                .unwrap()
                .insert("online".to_string(), json!(1));
        }
        let res: String = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(res)
    }
}
