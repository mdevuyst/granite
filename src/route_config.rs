use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub trait RouteHolder: Send + Sync {
    fn add_route(&self, route: RouteConfig);
    fn delete_route(&self, name: &str);
}

// TODO: See if the `http` crate already had something like this.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub enum Protocol {
    Http,
    Https,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Origin {
    pub host: String,
    pub port: u16,
    pub protocol: Protocol,
    pub host_header_override: Option<String>,
    pub sni: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct OriginGroup {
    pub origins: Vec<Origin>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct RouteConfig {
    pub name: String,
    pub customer: String,
    pub inbound_protocols: HashSet<Protocol>,
    pub hosts: Vec<String>,
    pub paths: Vec<String>,
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
            "inbound_protocols": [
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
            "origin_group": {
                "origins": [
                    {
                        "host": "origin1.com",
                        "port": 443,
                        "protocol": "Https",
                        "host_header_override": "foo.com",
                        "sni": "foo.com"
                    },
                    {
                        "host": "origin2.com",
                        "port": 80,
                        "protocol": "Http",
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
                inbound_protocols: HashSet::from([Protocol::Https, Protocol::Http]),
                hosts: vec!["example1.com".to_string(), "example2.com".to_string()],
                paths: vec!["/".to_string()],
                origin_group: OriginGroup {
                    origins: vec![
                        Origin {
                            host: "origin1.com".to_string(),
                            port: 443,
                            protocol: Protocol::Https,
                            host_header_override: Some("foo.com".to_string()),
                            sni: Some("foo.com".to_string()),
                        },
                        Origin {
                            host: "origin2.com".to_string(),
                            port: 80,
                            protocol: Protocol::Http,
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
