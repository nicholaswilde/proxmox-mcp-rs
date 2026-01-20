use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    // --- Metric Servers ---

    pub async fn get_metric_servers(&self) -> Result<Vec<Value>> {
        let path = "cluster/metrics/server";
        Ok(self.request(Method::GET, path, None).await?)
    }

    #[allow(dead_code)]
    pub async fn get_metric_server(&self, id: &str) -> Result<Value> {
        let path = format!("cluster/metrics/server/{}", id);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn create_metric_server(
        &self,
        id: &str,
        server_type: &str,
        params: &Value,
    ) -> Result<()> {
        let path = "cluster/metrics/server";
        let mut body = params.as_object().cloned().unwrap_or_default();
        body.insert("id".to_string(), serde_json::json!(id));
        body.insert("type".to_string(), serde_json::json!(server_type));

        self.request::<()>(Method::POST, path, Some(&serde_json::Value::Object(body)))
            .await?;
        Ok(())
    }

    pub async fn update_metric_server(&self, id: &str, params: &Value) -> Result<()> {
        let path = format!("cluster/metrics/server/{}", id);
        self.request::<()>(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_metric_server(&self, id: &str) -> Result<()> {
        let path = format!("cluster/metrics/server/{}", id);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }
}
