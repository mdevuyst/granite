use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub trait RouteHolder: Send + Sync {
    fn add_route(&self, route: RouteConfig);
    fn delete_route(&self, name: &str);
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub enum IncomingScheme {
    Http,
    Https,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash, Default)]
pub enum OutgoingScheme {
    Http,
    Https,

    #[default]
    MatchIncoming,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Origin {
    pub host: String,

    #[serde(default = "default_http_port")]
    pub http_port: u16,

    #[serde(default = "default_https_port")]
    pub https_port: u16,

    pub host_header_override: Option<String>,
    pub sni: Option<String>,

    #[serde(default = "default_weight")]
    pub weight: u16,
}

fn default_http_port() -> u16 {
    80
}

fn default_https_port() -> u16 {
    443
}

fn default_weight() -> u16 {
    10
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct OriginGroup {
    pub origins: Vec<Origin>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct RouteConfig {
    pub name: String,
    pub customer: String,
    pub incoming_schemes: HashSet<IncomingScheme>,
    pub hosts: Vec<String>,
    pub paths: Vec<String>,

    #[serde(default)]
    pub cache: bool,

    #[serde(default)]
    pub outgoing_scheme: OutgoingScheme,

    pub origin_group: OriginGroup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let json = r#"{
            "name": "route1",
            "customer": "customer1",
            "incoming_schemes": [
                "Http",
                "Https"
            ],
            "hosts": [
                "example1.com",
                "example2.com"
            ],
            "paths": [
                "/"
            ],
            "outgoing_scheme": "MatchIncoming",
            "origin_group": {
                "origins": [
                    {
                        "host": "origin1.com",
                        "http_port": 8080,
                        "weight": 10,
                        "host_header_override": "foo.com",
                        "sni": "foo.com"
                    },
                    {
                        "host": "origin2.com",
                        "http_port": 8080,
                        "https_port": 4433,
                        "weight": 20,
                        "sni": null
                    }
                ]
            }
        }"#;

        let route = serde_json::from_str::<RouteConfig>(json).unwrap();

        assert_eq!(
            RouteConfig {
                name: "route1".to_string(),
                customer: "customer1".to_string(),
                incoming_schemes: HashSet::from([IncomingScheme::Https, IncomingScheme::Http]),
                hosts: vec!["example1.com".to_string(), "example2.com".to_string()],
                paths: vec!["/".to_string()],
                cache: false,
                outgoing_scheme: OutgoingScheme::MatchIncoming,
                origin_group: OriginGroup {
                    origins: vec![
                        Origin {
                            host: "origin1.com".to_string(),
                            http_port: 8080,
                            https_port: 443,
                            weight: 10,
                            host_header_override: Some("foo.com".to_string()),
                            sni: Some("foo.com".to_string()),
                        },
                        Origin {
                            host: "origin2.com".to_string(),
                            http_port: 8080,
                            https_port: 4433,
                            weight: 20,
                            host_header_override: None,
                            sni: None,
                        },
                    ],
                },
            },
            route
        );
    }
}
