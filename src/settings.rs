use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct InstanceConfig {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub token_name: Option<String>,
    pub token_value: Option<String>,
    pub no_verify_ssl: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Settings {
    // Legacy single-instance fields
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub token_name: Option<String>,
    pub token_value: Option<String>,
    pub no_verify_ssl: Option<bool>,

    // New multi-instance field
    pub instances: Option<Vec<InstanceConfig>>,

    #[allow(dead_code)]
    pub log_level: Option<String>,
    #[allow(dead_code)]
    pub log_file_enable: Option<bool>,
    #[allow(dead_code)]
    pub log_dir: Option<String>,
    #[allow(dead_code)]
    pub log_filename: Option<String>,
    #[allow(dead_code)]
    pub log_rotate: Option<String>,
    pub server_type: Option<String>,
    pub http_host: Option<String>,
    pub http_port: Option<u16>,
    pub http_auth_token: Option<String>,
    pub lazy_mode: Option<bool>,
}

impl Settings {
    pub fn new(config_path: Option<&str>) -> Result<Self, ConfigError> {
        let mut s = Config::builder();

        // 1. Default config file "config.toml" (or json/yaml) in current directory
        // We make it optional so it doesn't fail if missing, UNLESS user specified a path.
        if let Some(path) = config_path {
            if Path::new(path).exists() {
                s = s.add_source(File::with_name(path));
            } else {
                // If user specifically asked for a config file and it's missing, we should probably fail?
                // The config crate will fail if required(true) is set.
                s = s.add_source(File::with_name(path).required(true));
            }
        } else {
            // Try default 'config' file in current dir, not required
            s = s.add_source(File::with_name("config").required(false));
        }

        // 2. Environment variables
        // Maps PROXMOX_HOST to host, PROXMOX_USER to user, etc.
        s = s.add_source(Environment::with_prefix("PROXMOX").separator("_"));

        s.build()?.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        let has_legacy = self.host.is_some() && self.user.is_some();
        let has_instances = self
            .instances
            .as_ref()
            .map(|i| !i.is_empty())
            .unwrap_or(false);

        if !has_legacy && !has_instances {
            return Err("Configuration missing: Provide either a single instance (host, user) or an 'instances' list.".to_string());
        }

        // Validate legacy if present
        if has_legacy {
            self.validate_instance(
                self.host.as_deref(),
                self.user.as_deref(),
                self.password.as_deref(),
                self.token_name.as_deref(),
                self.token_value.as_deref(),
            )?;
        }

        // Validate instances list if present
        if let Some(instances) = &self.instances {
            for (idx, inst) in instances.iter().enumerate() {
                if let Err(e) = self.validate_instance(
                    inst.host.as_deref(),
                    inst.user.as_deref(),
                    inst.password.as_deref(),
                    inst.token_name.as_deref(),
                    inst.token_value.as_deref(),
                ) {
                    return Err(format!("Instance {} error: {}", idx, e));
                }
            }
        }

        Ok(())
    }

    fn validate_instance(
        &self,
        host: Option<&str>,
        user: Option<&str>,
        password: Option<&str>,
        token_name: Option<&str>,
        token_value: Option<&str>,
    ) -> Result<(), String> {
        if host.is_none() || host.unwrap().is_empty() {
            return Err("Host is required".to_string());
        }
        if user.is_none() || user.unwrap().is_empty() {
            return Err("User is required".to_string());
        }

        let has_password = password.map(|s| !s.is_empty()).unwrap_or(false);
        let has_token = token_name.map(|s| !s.is_empty()).unwrap_or(false)
            && token_value.map(|s| !s.is_empty()).unwrap_or(false);

        if !has_password && !has_token {
            return Err("Either Password or API Token (name and value) is required".to_string());
        }

        if has_password && has_token {
            return Err("Provide either Password or API Token, not both".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;

    #[test]
    fn test_load_from_file() {
        let mut file = Builder::new().suffix(".toml").tempfile().unwrap();
        writeln!(
            file,
            "host = '1.2.3.4'\nuser = 'testuser'\npassword = 'pw'\nno_verify_ssl = true"
        )
        .unwrap();

        let path = file.path().to_str().unwrap();
        let settings = Settings::new(Some(path)).unwrap();

        assert_eq!(settings.host, Some("1.2.3.4".to_string()));
        assert_eq!(settings.user, Some("testuser".to_string()));
        assert_eq!(settings.password, Some("pw".to_string()));
        assert_eq!(settings.no_verify_ssl, Some(true));
    }

    #[test]
    fn test_validation() {
        let s = Settings {
            host: None,
            port: None,
            user: Some("u".into()),
            password: Some("p".into()),
            token_name: None,
            token_value: None,
            no_verify_ssl: Some(false),
            instances: None,
            log_level: None,
            log_file_enable: None,
            log_dir: None,
            log_filename: None,
            log_rotate: None,
            server_type: None,
            http_host: None,
            http_port: None,
            http_auth_token: None,
            lazy_mode: None,
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn test_validation_token() {
        let s = Settings {
            host: Some("h".into()),
            port: None,
            user: Some("u".into()),
            password: None,
            token_name: Some("t".into()),
            token_value: Some("v".into()),
            no_verify_ssl: Some(false),
            instances: None,
            log_level: None,
            log_file_enable: None,
            log_dir: None,
            log_filename: None,
            log_rotate: None,
            server_type: None,
            http_host: None,
            http_port: None,
            http_auth_token: None,
            lazy_mode: None,
        };
        assert!(s.validate().is_ok());
    }

    #[test]
    fn test_validation_exclusive() {
        let s = Settings {
            host: Some("h".into()),
            port: None,
            user: Some("u".into()),
            password: Some("p".into()),
            token_name: Some("t".into()),
            token_value: Some("v".into()),
            no_verify_ssl: Some(false),
            instances: None,
            log_level: None,
            log_file_enable: None,
            log_dir: None,
            log_filename: None,
            log_rotate: None,
            server_type: None,
            http_host: None,
            http_port: None,
            http_auth_token: None,
            lazy_mode: None,
        };
        assert!(s.validate().is_err());
    }
}
