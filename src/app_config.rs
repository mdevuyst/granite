//! Static application configuration read from a YAML file at application startup.

use log::debug;
use pingora::prelude::*;
use pingora::{Error, OrErr, Result};
use serde::{Deserialize, Serialize};
use std::fs;

/// The top-level configuration for the application.  The configuration is further broken down into
/// `proxy`, `cache`, and `api` sections.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
#[serde(default)]
pub struct AppConfig {
    pub proxy: ProxyConfig,
    pub cache: CacheConfig,
    pub api: ApiConfig,
}

/// Proxy settings.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct ProxyConfig {
    /// A list of socket addresses to bind to for HTTP traffic.
    /// Format of each address is `ip:port`.  E.g., `0.0.0.0:80`.
    pub http_bind_addrs: Vec<String>,

    /// A list of socket addresses to bind to for HTTP traffic.
    /// Format of each address is `ip:port`.  E.g., `0.0.0.0:443`.
    pub https_bind_addrs: Vec<String>,

    /// The amount of time (in seconds) an origin is marked down if it fails to connect.
    pub origin_down_time: u64,

    /// The maximum number of times to retry connecting to an origin.
    pub connection_retry_limit: u16,
}

/// Cache settings.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct CacheConfig {
    /// The maximum size (in bytes) the cache is allowed to grow to.  If it gets larger, the least
    /// recently used items will be evicted.
    pub max_size: usize,
}

/// Settings for the config API service.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct ApiConfig {
    /// The socket address to bind to.  Format is `ip:port`.  E.g., `0.0.0.0:5000`.
    pub bind_addr: String,

    /// Whether to enable TLS for the API service.
    pub tls: bool,

    /// If TLS is enabled, the path to the certificate file.
    pub cert: Option<String>,

    /// If TLS is enabled, the path to the private key file.
    pub key: Option<String>,

    /// Whether to enable mutual TLS for the API service.
    pub mutual_tls: bool,

    /// If mutual TLS is enabled, the path to the client certificate file.
    /// Only clients presenting this certificate will be allowed to connect.
    pub client_cert: Option<String>,
}

impl AppConfig {
    /// Load the configuration from a YAML file.
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

    /// Parse the configuration from a YAML string.
    pub fn from_yaml(conf_str: &str) -> Result<Self> {
        let conf: AppConfig = serde_yaml::from_str(conf_str).or_err_with(ReadError, || {
            format!("Unable to parse yaml conf {conf_str}")
        })?;
        conf.validate()
    }

    /// Validate the configuration.
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
    /// By default, bind to all interfaces on port 8080 for HTTP and 4433 for HTTPS.
    fn default() -> Self {
        ProxyConfig {
            http_bind_addrs: vec!["0.0.0.0:8080".to_string()],
            https_bind_addrs: vec!["0.0.0.0:4433".to_string()],
            origin_down_time: 10,
            connection_retry_limit: 1,
        }
    }
}

impl Default for CacheConfig {
    /// The default maximum cache size is 100 MB.
    fn default() -> Self {
        CacheConfig {
            max_size: 100 * 1024 * 1024,
        }
    }
}

impl Default for ApiConfig {
    /// By default, bind to all interfaces on port 5000 with no TLS.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_yaml() {
        let yaml = r#"
            proxy:
              http_bind_addrs:
                - 127.0.0.1:81
                - 127.0.0.2:82
              https_bind_addrs:
                - 0.0.0.0:443
              origin_down_time: 5
              connection_retry_limit: 2
            cache:
              max_size: 5000000
            api:
              bind_addr: 127.0.1.5:6000
              tls: true
              cert: /path/to/api.crt
              key: /path/to/api.key
              mutual_tls: true
              client_cert: /path/to/client.crt
        "#;
        let conf = AppConfig::from_yaml(yaml).unwrap();
        assert_eq!(
            conf,
            AppConfig {
                proxy: ProxyConfig {
                    http_bind_addrs: vec!["127.0.0.1:81".to_string(), "127.0.0.2:82".to_string()],
                    https_bind_addrs: vec!["0.0.0.0:443".to_string()],
                    origin_down_time: 5,
                    connection_retry_limit: 2,
                },
                cache: CacheConfig { max_size: 5000000 },
                api: ApiConfig {
                    bind_addr: "127.0.1.5:6000".to_string(),
                    tls: true,
                    cert: Some("/path/to/api.crt".to_string()),
                    key: Some("/path/to/api.key".to_string()),
                    mutual_tls: true,
                    client_cert: Some("/path/to/client.crt".to_string()),
                }
            }
        );
    }

    #[test]
    fn missing_cert() {
        let yaml = r#"
            api:
              tls: true
              key: /path/to/api.key
        "#;
        assert!(AppConfig::from_yaml(yaml).is_err());
    }

    #[test]
    fn missing_key() {
        let yaml = r#"
            api:
              tls: true
              cert: /path/to/api.crt
        "#;
        assert!(AppConfig::from_yaml(yaml).is_err());
    }

    #[test]
    fn missing_client_cert() {
        let yaml = r#"
            api:
              tls: true
              cert: /path/to/api.crt
              key: /path/to/api.key
              mutual_tls: true
        "#;
        assert!(AppConfig::from_yaml(yaml).is_err());
    }
}
