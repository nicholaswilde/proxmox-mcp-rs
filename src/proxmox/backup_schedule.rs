use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::Value;

impl ProxmoxClient {
    pub async fn get_backup_schedules(&self) -> Result<Vec<Value>> {
        let path = "cluster/backup";
        Ok(self.request(Method::GET, path, None).await?)
    }

    pub async fn create_backup_schedule(&self, params: &Value) -> Result<()> {
        let path = "cluster/backup";
        self.request::<()>(Method::POST, path, Some(params)).await?;
        Ok(())
    }

    pub async fn update_backup_schedule(&self, id: &str, params: &Value) -> Result<()> {
        let path = format!("cluster/backup/{}", id);
        self.request::<()>(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_backup_schedule(&self, id: &str) -> Result<()> {
        let path = format!("cluster/backup/{}", id);
        self.request::<()>(Method::DELETE, &path, None).await?;
        Ok(())
    }
}
