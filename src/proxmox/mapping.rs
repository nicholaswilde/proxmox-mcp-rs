use super::client::ProxmoxClient;
use crate::proxmox::error::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    // --- PCI Mappings ---

    pub async fn get_pci_mappings(&self) -> Result<Vec<Value>> {
        let path = "cluster/mapping/pci";
        self.request(Method::GET, path, None).await
    }

    pub async fn create_pci_mapping(&self, id: &str, params: &Value) -> Result<()> {
        let path = "cluster/mapping/pci";
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("id".to_string(), serde_json::json!(id));
        self.request::<()>(Method::POST, path, Some(&Value::Object(body)))
            .await?;
        Ok(())
    }

    pub async fn update_pci_mapping(&self, id: &str, params: &Value) -> Result<()> {
        let path = format!("cluster/mapping/pci/{}", id);
        self.request::<()>(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_pci_mapping(&self, id: &str) -> Result<()> {
        let path = format!("cluster/mapping/pci/{}", id);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }

    // --- USB Mappings ---

    pub async fn get_usb_mappings(&self) -> Result<Vec<Value>> {
        let path = "cluster/mapping/usb";
        self.request(Method::GET, path, None).await
    }

    pub async fn create_usb_mapping(&self, id: &str, params: &Value) -> Result<()> {
        let path = "cluster/mapping/usb";
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("id".to_string(), serde_json::json!(id));
        self.request::<()>(Method::POST, path, Some(&Value::Object(body)))
            .await?;
        Ok(())
    }

    pub async fn update_usb_mapping(&self, id: &str, params: &Value) -> Result<()> {
        let path = format!("cluster/mapping/usb/{}", id);
        self.request::<()>(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_usb_mapping(&self, id: &str) -> Result<()> {
        let path = format!("cluster/mapping/usb/{}", id);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }
}
