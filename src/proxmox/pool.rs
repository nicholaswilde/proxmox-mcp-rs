use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_pools(&self) -> Result<Vec<Value>> {
        Ok(self.request(Method::GET, "pools", None).await?)
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
        Ok(self.request(Method::GET, &path, None).await?)
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
}
