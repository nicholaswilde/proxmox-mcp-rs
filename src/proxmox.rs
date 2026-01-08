use anyhow::{Context, Result};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use log::info;
use url::Url;

#[derive(Clone)]
pub struct ProxmoxClient {
    client: Client,
    base_url: Url,
    ticket: Option<String>,
    csrf_token: Option<String>,
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

impl ProxmoxClient {
    pub fn new(host: &str, verify_ssl: bool) -> Result<Self> {
        let base_url = Url::parse(&format!("https://{}/api2/json/", host))
            .context("Invalid host URL")?;

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
        })
    }

    pub async fn login(&mut self, user: &str, password: &str) -> Result<()> {
        let url = self.base_url.join("access/ticket")?;
        let params = [("username", user), ("password", password)];

        let resp = self.client.post(url)
            .form(&params)
            .send()
            .await
            .context("Login request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Login failed: {} - {}", status, text);
        }

        let body: TicketResponse = resp.json().await.context("Failed to parse login response")?;
        
        self.ticket = Some(body.data.ticket);
        self.csrf_token = Some(body.data.csrf_token);
        
        info!("Successfully logged in as {}", user);
        Ok(())
    }

    async fn request<T: serde::de::DeserializeOwned>(&self, method: Method, path: &str, body: Option<&Value>) -> Result<T> {
        let url = self.base_url.join(path)?;
        let mut req = self.client.request(method, url);

        if let Some(token) = &self.csrf_token {
             req = req.header("CSRFPreventionToken", token);
        }
        
        // Manually add cookie if we have a ticket
        if let Some(ticket) = &self.ticket {
            req = req.header("Cookie", format!("PVEAuthCookie={}", ticket));
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

    pub async fn get_vms(&self, node: &str) -> Result<Vec<VmInfo>> {
        // qemu and lxc are separate.
        // GET /nodes/{node}/qemu
        // GET /nodes/{node}/lxc
        // We can aggregate.
        
        let qemu: Vec<VmInfo> = self.request(Method::GET, &format!("nodes/{}/qemu", node), None).await.unwrap_or_default();
        let lxc: Vec<VmInfo> = self.request(Method::GET, &format!("nodes/{}/lxc", node), None).await.unwrap_or_default();

        let mut all = qemu;
        all.extend(lxc);
        // Fill in 'node' field since API might not return it in the list context of a specific node
        for vm in &mut all {
            vm.node = Some(node.to_string());
        }
        Ok(all)
    }

    pub async fn get_all_vms(&self) -> Result<Vec<VmInfo>> {
        let nodes = self.get_nodes().await?;
        let mut all_vms = Vec::new();
        
        for node in nodes {
            if let Some(node_name) = node.get("node").and_then(|n| n.as_str()) {
                let vms = self.get_vms(node_name).await?;
                all_vms.extend(vms);
            }
        }
        Ok(all_vms)
    }

    pub async fn vm_action(&self, node: &str, vmid: i64, action: &str, vm_type: Option<&str>) -> Result<String> {
        // Infer type if missing? safer to require or try both.
        // API paths: /nodes/{node}/qemu/{vmid}/status/{action} or /lxc/...
        // We can try qemu first, if fails try lxc? Or check list.
        // For efficiency, let's assume caller provides type or we find it.
        
        let type_path = if let Some(t) = vm_type {
            t // "qemu" or "lxc"
        } else {
            // naive check: try qemu
             "qemu" 
        };

        let path = format!("nodes/{}/{}/{}/status/{}", node, type_path, vmid, action);
        // Actions like start, stop, shutdown, reset, suspend, resume
        
        // Returns UPID usually
        let res: String = self.request(Method::POST, &path, None).await?;
        Ok(res)
    }

    pub async fn create_resource(&self, node: &str, resource_type: &str, params: &Value) -> Result<String> {
        let path = format!("nodes/{}/{}", node, resource_type);
        let res: String = self.request(Method::POST, &path, Some(params)).await?;
        Ok(res)
    }

    pub async fn delete_resource(&self, node: &str, vmid: i64, resource_type: &str) -> Result<String> {
        let path = format!("nodes/{}/{}/{}", node, resource_type, vmid);
        let res: String = self.request(Method::DELETE, &path, None).await?;
        Ok(res)
    }
}
