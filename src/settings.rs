use config::{Config, ConfigError, File, Environment};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Settings {
    pub host: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub no_verify_ssl: Option<bool>,
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
        if self.host.is_none() || self.host.as_ref().unwrap().is_empty() {
            return Err("Host is required".to_string());
        }
        if self.user.is_none() || self.user.as_ref().unwrap().is_empty() {
            return Err("User is required".to_string());
        }
        if self.password.is_none() || self.password.as_ref().unwrap().is_empty() {
            return Err("Password is required".to_string());
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
        writeln!(file, "host = '1.2.3.4'\nuser = 'testuser'\npassword = 'pw'\nno_verify_ssl = true").unwrap();
        
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
            user: Some("u".into()),
            password: Some("p".into()),
            no_verify_ssl: Some(false),
        };
        assert!(s.validate().is_err());
    }
}
