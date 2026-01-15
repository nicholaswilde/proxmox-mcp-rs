use super::client::ProxmoxClient;
use anyhow::Result;
use reqwest::Method;
use serde_json::{json, Value};

impl ProxmoxClient {
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
