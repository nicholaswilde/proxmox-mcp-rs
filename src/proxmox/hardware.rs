use super::client::ProxmoxClient;
use crate::proxmox::error::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    pub async fn get_pci_devices(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/hardware/pci", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn get_usb_devices(&self, node: &str) -> Result<Vec<Value>> {
        let path = format!("nodes/{}/hardware/usb", node);
        Ok(self.request(Method::GET, &path, None).await?)
    }
}
