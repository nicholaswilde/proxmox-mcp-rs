use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    pub async fn get_replication_jobs(&self) -> Result<Vec<Value>> {
        self.request(Method::GET, "cluster/replication", None).await
    }

    pub async fn create_replication_job(
        &self,
        id: &str,
        target: &str,
        schedule: Option<&str>,
        rate: Option<f64>,
        comment: Option<&str>,
        enable: Option<bool>,
    ) -> Result<()> {
        let mut params = json!({
            "id": id,
            "target": target,
            "type": "local" // Usually 'local' for ZFS replication
        });

        if let Some(s) = schedule {
            params
                .as_object_mut()
                .unwrap()
                .insert("schedule".to_string(), json!(s));
        }
        if let Some(r) = rate {
            params
                .as_object_mut()
                .unwrap()
                .insert("rate".to_string(), json!(r));
        }
        if let Some(c) = comment {
            params
                .as_object_mut()
                .unwrap()
                .insert("comment".to_string(), json!(c));
        }
        if let Some(e) = enable {
            params
                .as_object_mut()
                .unwrap()
                .insert("disable".to_string(), json!(if e { 0 } else { 1 }));
        }

        let _: Value = self
            .request(Method::POST, "cluster/replication", Some(&params))
            .await?;
        Ok(())
    }

    pub async fn update_replication_job(&self, id: &str, params: &Value) -> Result<()> {
        let path = format!("cluster/replication/{}", id);
        let _: Value = self.request(Method::PUT, &path, Some(params)).await?;
        Ok(())
    }

    pub async fn delete_replication_job(&self, id: &str) -> Result<()> {
        let path = format!("cluster/replication/{}", id);
        let _: Value = self.request(Method::DELETE, &path, None).await?;
        Ok(())
    }

    // Usually POST /nodes/{node}/replication/{id}/schedule_now implies running it
    // But the API path is often cluster/replication for config, and per-node for status/log.
    // To RUN a job immediately:
    // It seems there isn't a direct "run now" in the cluster/replication API easily documented?
    // Actually `pvesr run --id <jobid>` is the CLI way.
    // API equivalent might be tricky or implicit via schedule update.
    // Wait, the API docs show GET /nodes/{node}/replication/{id}/schedule_now is not a standard endpoint.
    // However, users can force a run by `pvesr`.
    // Let's stick to CRUD for now, running might require agent exec or specific knowledge.
    // Found it: POST /nodes/{node}/replication/{id}/schedule_now is NOT standard.
    // We will omit 'run_replication_job' for now unless we confirm the endpoint.
    // Update: Some docs suggest POST /cluster/replication/{id} with specific params? No.
    // We will stick to CRUD.
}
