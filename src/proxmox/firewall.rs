use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
    fn resolve_alias_base_path(&self, level: &str, node: Option<&str>) -> Result<String> {
        match level {
            "cluster" => Ok("cluster/firewall/aliases".to_string()),
            "node" => {
                let node =
                    node.ok_or_else(|| anyhow::anyhow!("Node name required for node level"))?;
                Ok(format!("nodes/{}/firewall/aliases", node))
            }
            _ => Err(anyhow::anyhow!(
                "Invalid level: must be 'cluster' or 'node'"
            )),
        }
    }

    pub async fn get_aliases(&self, level: &str, node: Option<&str>) -> Result<Vec<Value>> {
        let path = self.resolve_alias_base_path(level, node)?;
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn create_alias(
        &self,
        level: &str,
        node: Option<&str>,
        name: &str,
        cidr: &str,
        comment: Option<&str>,
    ) -> Result<()> {
        let path = self.resolve_alias_base_path(level, node)?;
        let mut params = json!({
            "name": name,
            "cidr": cidr,
        });
        if let Some(c) = comment {
            params
                .as_object_mut()
                .unwrap()
                .insert("comment".to_string(), json!(c));
        }

        let _: Value = self.request(Method::POST, &path, Some(&params)).await?;
        Ok(())
    }

    pub async fn update_alias(
        &self,
        level: &str,
        node: Option<&str>,
        name: &str,
        cidr: &str,
        comment: Option<&str>,
    ) -> Result<()> {
        let base_path = self.resolve_alias_base_path(level, node)?;
        let path = format!("{}/{}", base_path, name);

        let mut params = json!({
            "cidr": cidr,
        });
        if let Some(c) = comment {
            params
                .as_object_mut()
                .unwrap()
                .insert("comment".to_string(), json!(c));
        }

        let _: Value = self.request(Method::PUT, &path, Some(&params)).await?;
        Ok(())
    }

    pub async fn delete_alias(&self, level: &str, node: Option<&str>, name: &str) -> Result<()> {
        let base_path = self.resolve_alias_base_path(level, node)?;
        let path = format!("{}/{}", base_path, name);

        let _: Value = self.request(Method::DELETE, &path, None).await?;
        Ok(())
    }

    // --- Security Groups ---

    pub async fn get_security_groups(&self) -> Result<Vec<Value>> {
        Ok(self.request(Method::GET, "cluster/firewall/groups", None).await?)
    }

    pub async fn create_security_group(&self, name: &str, comment: Option<&str>) -> Result<()> {
        let mut params = json!({ "group": name });
        if let Some(c) = comment {
            params.as_object_mut().unwrap().insert("comment".to_string(), json!(c));
        }
        let _: Value = self.request(Method::POST, "cluster/firewall/groups", Some(&params)).await?;
        Ok(())
    }

    pub async fn delete_security_group(&self, name: &str) -> Result<()> {
        let path = format!("cluster/firewall/groups/{}", name);
        let _: Value = self.request(Method::DELETE, &path, None).await?;
        Ok(())
    }

    pub async fn get_security_group_rules(&self, name: &str) -> Result<Vec<Value>> {
        let path = format!("cluster/firewall/groups/{}", name);
        Ok(self.request(Method::GET, &path, None).await?)
    }

    pub async fn add_security_group_rule(&self, name: &str, rule: &Value) -> Result<()> {
        let path = format!("cluster/firewall/groups/{}", name);
        let _: Value = self.request(Method::POST, &path, Some(rule)).await?;
        Ok(())
    }
}
