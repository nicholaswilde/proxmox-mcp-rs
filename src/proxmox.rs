use anyhow::{Context, Result};
use log::info;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

#[derive(Clone)]
pub struct ProxmoxClient {
    client: Client,
    base_url: Url,
    ticket: Option<String>,
    csrf_token: Option<String>,
    api_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TicketResponse {
    data: TicketData,
}

#[derive(Deserialize, Debug)]
struct TicketData {
    ticket: String,
    #[serde(rename = "CSRFPreventionToken")]
    csrf_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VmInfo {
    pub vmid: i64, // Proxmox uses integer IDs mostly, but sometimes strings. i64 is safe.
    pub name: Option<String>,
    pub status: String,
    pub node: Option<String>,
    #[serde(rename = "type")]
    pub vm_type: Option<String>, // qemu or lxc
}

#[derive(Deserialize, Debug)]
pub struct ClusterResource {
    pub vmid: Option<i64>,
    pub node: String,
    #[serde(rename = "type")]
    pub res_type: String,
    pub status: Option<String>,
    pub name: Option<String>,
}

impl ProxmoxClient {
    pub fn new(host: &str, port: u16, verify_ssl: bool) -> Result<Self> {
        let scheme = if host.starts_with("http://") {
            "http"
        } else {
            "https"
        };

        let host_cleaned = if let Some(stripped) = host.strip_prefix("http://") {
            stripped
        } else if let Some(stripped) = host.strip_prefix("https://") {
            stripped
        } else {
            host
        };
        let host_cleaned = host_cleaned.trim_end_matches('/');

        let url_str = format!("{}://{}:{}/api2/json/", scheme, host_cleaned, port);

        let base_url = Url::parse(&url_str).context("Invalid host URL")?;

        let client = Client::builder()
            .danger_accept_invalid_certs(!verify_ssl)
            .cookie_store(true)
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self {
            client,
            base_url,
            ticket: None,
            csrf_token: None,
            api_token: None,
        })
    }

    pub fn set_api_token(&mut self, user: &str, token_name: &str, token_value: &str) {
        // Format: PVEAPIToken=USER@REALM!TOKENID=UUID
        self.api_token = Some(format!(
            "PVEAPIToken={}!{}={}",
            user, token_name, token_value
        ));
    }

    pub async fn login(&mut self, user: &str, password: &str) -> Result<()> {
        let url = self.base_url.join("access/ticket")?;
        let params = [("username", user), ("password", password)];

        let resp = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await
            .context("Login request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Login failed: {} - {}", status, text);
        }

        let body: TicketResponse = resp
            .json()
            .await
            .context("Failed to parse login response")?;

        self.ticket = Some(body.data.ticket);
        self.csrf_token = Some(body.data.csrf_token);

        info!("Successfully logged in as {}", user);
        Ok(())
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let mut req = self.client.request(method, url);

        if let Some(token) = &self.api_token {
            req = req.header("Authorization", token);
        } else {
            if let Some(token) = &self.csrf_token {
                req = req.header("CSRFPreventionToken", token);
            }

            // Manually add cookie if we have a ticket
            if let Some(ticket) = &self.ticket {
                req = req.header("Cookie", format!("PVEAuthCookie={}", ticket));
            }
        }

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Request to {} failed: {} - {}", path, status, text);
        }

        // Proxmox wraps response in { "data": ... } usually.
        let v: Value = resp.json().await?;
        if let Some(data) = v.get("data") {
            serde_json::from_value(data.clone()).context("Failed to deserialize data field")
        } else {
            // Sometimes it might not have data wrapper (unlikely for successful api calls)
            serde_json::from_value(v).context("Failed to deserialize response")
        }
    }

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

    pub async fn vm_action(
        &self,
        node: &str,
        vmid: i64,
        action: &str,
        vm_type: Option<&str>,
    ) -> Result<String> {
        // Infer type if missing? safer to require or try both.
        // API paths: /nodes/{node}/qemu/{vmid}/status/{action} or /lxc/...
        // We can try qemu first, if fails try lxc? Or check list.
        // For efficiency, let's assume caller provides type or we find it.

        let type_path = vm_type.unwrap_or("qemu");

        let path = format!("nodes/{}/{}/{}/status/{}", node, type_path, vmid, action);
        // Actions like start, stop, shutdown, reset, suspend, resume

        // Returns UPID usually
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
        self.request(Method::GET, &path, None).await
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

    // --- Hardware Configuration ---

    #[allow(clippy::too_many_arguments)]
    pub async fn add_virtual_disk(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        device: &str, // e.g., "scsi0", "virtio0", "rootfs" (for LXC?)
        storage: &str,
        size_gb: u64,
        format: Option<&str>,        // raw, qcow2, etc. (optional)
        extra_options: Option<&str>, // e.g. "discard=on"
    ) -> Result<()> {
        // Construct value: "storage:size_gb"
        // For LXC "rootfs": "storage:size_gb" (but usually created at start).
        // For QEMU: "storage:size_gb,format=..."

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
        device: &str,        // "net0", "net1"
        model: Option<&str>, // "virtio", "e1000" (QEMU only)
        bridge: &str,
        mac: Option<&str>,
        extra_options: Option<&str>,
    ) -> Result<()> {
        // QEMU: net0: virtio=AABB...,bridge=vmbr0
        // LXC: net0: name=eth0,bridge=vmbr0,...

        let mut value = String::new();

        if resource_type == "qemu" {
            let m = model.unwrap_or("virtio");
            // virtio=MAC (if mac provided) or just virtio
            if let Some(addr) = mac {
                value.push_str(&format!("{}={},bridge={}", m, addr, bridge));
            } else {
                value.push_str(&format!("{},bridge={}", m, bridge));
            }
        } else {
            // LXC
            value.push_str(&format!(
                "name=eth{},bridge={}",
                device.replace("net", ""),
                bridge
            ));
            if let Some(addr) = mac {
                value.push_str(&format!(",hwaddr={}", addr));
            }
            if let Some(m) = model {
                // LXC doesn't really use model in the same way, usually 'type=veth' is default
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

    // --- QEMU Guest Agent ---

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
        // agent/exec returns a PID. We usually need to wait for it or return the PID.
        // For MCP, simple exec is often preferred. But PVE agent exec is async inside the guest.
        // Returns { "pid": <int> }.
        // We might want a separate tool to wait? Or just return the PID.
        // Let's return the result structure from PVE.

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

    // --- Snapshot Management ---

    pub async fn get_snapshots(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
    ) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/{}/{}/snapshot", node, resource_type, vmid);
        self.request(Method::GET, &path, None).await
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

    // --- Clone and Migrate ---

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
                    // Check 'vmid' field if available
                    if let Some(bid) = b.get("vmid").and_then(|v| v.as_i64()) {
                        return bid == id;
                    }
                    // Fallback: parse from volid (e.g., backup/vzdump-qemu-100-...)
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

    // Restore behaves differently for QEMU (qmrestore) and LXC (pct restore)
    // usually POST /nodes/{node}/{type} with archive={volid} and vmid={vmid}
    pub async fn restore_backup(
        &self,
        node: &str,
        vmid: i64,
        resource_type: &str,
        archive: &str,
        storage: Option<&str>,
        force: Option<bool>,
    ) -> Result<String> {
        // resource_type should be "qemu" or "lxc"
        let path = format!("nodes/{}/{}", node, resource_type);

        let mut params = json!({
            "vmid": vmid,
            "archive": archive,
            "restore": 1
        });

        // For LXC, 'restore' param is implicitly handled by having 'archive' in create call?
        // Actually for QEMU: POST /nodes/{node}/qemu with 'archive' creates from backup.
        // For LXC: POST /nodes/{node}/lxc with 'ostemplate' as the backup file creates from backup.
        // But 'ostemplate' field is used for templates AND backups in LXC create.

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

    // --- Task Monitoring ---

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

    // --- Network Management ---

    pub async fn get_network_interfaces(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/network", node);
        self.request(Method::GET, &path, None).await
    }

    // --- Storage & ISO Management ---

    pub async fn get_storage_list(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/storage", node);
        self.request(Method::GET, &path, None).await
    }

    pub async fn get_cluster_storage(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "storage", None).await
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

    // --- Cluster Management ---

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

    // --- Firewall Management ---

    pub async fn get_firewall_rules(
        &self,
        node: Option<&str>,
        vmid: Option<i64>,
    ) -> Result<Vec<Value>> {
        let path = if let (Some(n), Some(id)) = (node, vmid) {
            // Determine type first (hacky, but we need type for path).
            // Actually, we usually need the type to construct the path correctly:
            // /nodes/{node}/{type}/{vmid}/firewall/rules
            // We can try to find the VM location to get the type.
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

    // --- Statistics (RRD) ---

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

        self.request(Method::GET, &path, None).await
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

        self.request(Method::GET, &path, None).await
    }

    // --- User Management ---

    pub async fn get_users(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "access/users", None).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_user(
        &self,
        userid: &str,
        password: &str,
        email: Option<&str>,
        firstname: Option<&str>,
        lastname: Option<&str>,
        expire: Option<i64>,
        enable: Option<bool>,
        comment: Option<&str>,
        groups: Option<Vec<String>>,
    ) -> Result<()> {
        let mut params = json!({
            "userid": userid,
            "password": password,
        });

        if let Some(v) = email {
            params
                .as_object_mut()
                .unwrap()
                .insert("email".to_string(), json!(v));
        }
        if let Some(v) = firstname {
            params
                .as_object_mut()
                .unwrap()
                .insert("firstname".to_string(), json!(v));
        }
        if let Some(v) = lastname {
            params
                .as_object_mut()
                .unwrap()
                .insert("lastname".to_string(), json!(v));
        }
        if let Some(v) = expire {
            params
                .as_object_mut()
                .unwrap()
                .insert("expire".to_string(), json!(v));
        }
        if let Some(v) = enable {
            params
                .as_object_mut()
                .unwrap()
                .insert("enable".to_string(), json!(if v { 1 } else { 0 }));
        }
        if let Some(v) = comment {
            params
                .as_object_mut()
                .unwrap()
                .insert("comment".to_string(), json!(v));
        }
        if let Some(v) = groups {
            params
                .as_object_mut()
                .unwrap()
                .insert("groups".to_string(), json!(v.join(",")));
        }

        self.request(Method::POST, "access/users", Some(&params))
            .await
    }

    pub async fn delete_user(&self, userid: &str) -> Result<()> {
        let path = format!("access/users/{}", userid);
        self.request(Method::DELETE, &path, None).await
    }

    // --- Pool Management ---

    pub async fn get_pools(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "pools", None).await
    }

    pub async fn create_pool(&self, poolid: &str, comment: Option<&str>) -> Result<()> {
        let mut params = json!({ "poolid": poolid });
        if let Some(c) = comment {
            params
                .as_object_mut()
                .unwrap()
                .insert("comment".to_string(), json!(c));
        }
        let _: Value = self.request(Method::POST, "pools", Some(&params)).await?;
        Ok(())
    }

    pub async fn get_pool_details(&self, poolid: &str) -> Result<Value> {
        let path = format!("pools/{}", poolid);
        self.request(Method::GET, &path, None).await
    }

    pub async fn update_pool(&self, poolid: &str, params: &Value) -> Result<()> {
        let path = format!("pools/{}", poolid);
        let _: Value = self.request(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_pool(&self, poolid: &str) -> Result<()> {
        let path = format!("pools/{}", poolid);
        let _: Value = self.request(Method::DELETE, &path, None).await?;
        Ok(())
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

    // --- Roles & ACL Management ---

    pub async fn get_roles(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "access/roles", None).await
    }

    pub async fn create_role(&self, roleid: &str, privileges: &str) -> Result<()> {
        let params = json!({ "roleid": roleid, "privs": privileges });
        let _: Value = self
            .request(Method::POST, "access/roles", Some(&params))
            .await?;
        Ok(())
    }

    pub async fn update_role(&self, roleid: &str, privileges: &str, append: bool) -> Result<()> {
        let path = format!("access/roles/{}", roleid);
        let mut params = json!({ "privs": privileges });
        if append {
            params
                .as_object_mut()
                .unwrap()
                .insert("append".to_string(), json!(1));
        }
        let _: Value = self.request(Method::PUT, &path, Some(&params)).await?;
        Ok(())
    }

    pub async fn delete_role(&self, roleid: &str) -> Result<()> {
        let path = format!("access/roles/{}", roleid);
        let _: Value = self.request(Method::DELETE, &path, None).await?;
        Ok(())
    }

    pub async fn get_acls(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "access/acl", None).await
    }

    pub async fn update_acl(&self, path: &str, params: &Value) -> Result<()> {
        let mut full_params = params
            .as_object()
            .ok_or(anyhow::anyhow!("Params must be object"))?
            .clone();
        full_params.insert("path".to_string(), json!(path));
        let _: Value = self
            .request(Method::PUT, "access/acl", Some(&Value::Object(full_params)))
            .await?;
        Ok(())
    }
}
