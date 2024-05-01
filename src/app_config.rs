use log::debug;
use pingora::prelude::*;
use pingora::{Error, OrErr, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
#[serde(default)]
pub struct AppConfig {
    pub proxy: ProxyConfig,
    pub cache: CacheConfig,
    pub api: ApiConfig,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct ProxyConfig {
    pub http_bind_addrs: Vec<String>,
    pub https_bind_addrs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct CacheConfig {
    pub max_size: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct ApiConfig {
    pub bind_addr: String,
    pub tls: bool,
    pub cert: Option<String>,
    pub key: Option<String>,
    pub mutual_tls: bool,
    pub client_cert: Option<String>,
}

impl AppConfig {
    pub fn load_from_yaml<P>(path: P) -> Result<Self>
    where
        P: AsRef<std::path::Path> + std::fmt::Display,
    {
        let conf_str = fs::read_to_string(&path).or_err_with(ReadError, || {
            format!("Unable to read conf file from {path}")
        })?;
        debug!("Conf file read from {path}");
        Self::from_yaml(&conf_str)
    }

    pub fn from_yaml(conf_str: &str) -> Result<Self> {
        let conf: AppConfig = serde_yaml::from_str(conf_str).or_err_with(ReadError, || {
            format!("Unable to parse yaml conf {conf_str}")
        })?;
        conf.validate()
    }

    pub fn validate(self) -> Result<Self> {
        if self.api.tls {
            if self.api.cert.is_none() {
                return Err(Error::new_str("API: cert is required when tls is enabled"));
            }
            if self.api.key.is_none() {
                return Err(Error::new_str("API: key is required when tls is enabled"));
            }
        }
        if self.api.mutual_tls {
            if !self.api.tls {
                return Err(Error::new_str(
                    "API: tls must be enabled if mutual_tls is enabled",
                ));
            }
            if self.api.client_cert.is_none() {
                return Err(Error::new_str(
                    "API: client cert is required when mutual_tls is enabled",
                ));
            }
        }
        Ok(self)
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        ProxyConfig {
            http_bind_addrs: vec!["0.0.0.0:8080".to_string()],
            https_bind_addrs: vec!["0.0.0.0:4433".to_string()],
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            max_size: 100 * 1024 * 1024,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            bind_addr: "0.0.0.0:5000".to_string(),
            tls: false,
            cert: None,
            key: None,
            mutual_tls: false,
            client_cert: None,
        }
    }
}
