use serde::{Deserialize, Serialize};

pub trait RouteHolder: Send + Sync {
    fn add_route(&self, route: Route);
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Origin {
    pub host: String,
    pub port: u16,
    pub protocol: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct OriginGroup {
    pub origins: Vec<Origin>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Route {
    pub customer: String,
    pub hosts: Vec<String>,
    pub paths: Vec<String>,
    pub protocols: Vec<String>,
    pub origin_group: OriginGroup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let route = r#"{
            "customer": "customer1",
            "hosts": ["example1.com", "example2.com"],
            "paths": ["/"],
            "protocols": ["http"],
            "origin_group": {
                "origins": [
                    {
                        "host": "origin1.com",
                        "port": 80,
                        "protocol": "http"
                    },
                    {
                        "host": "origin2.com",
                        "port": 80,
                        "protocol": "http"
                    }
                ]
            }
        }"#;

        let route = serde_json::from_str::<Route>(route).unwrap();

        assert_eq!(
            Route {
                customer: "customer1".to_string(),
                hosts: vec!["example1.com".to_string(), "example2.com".to_string()],
                paths: vec!["/".to_string()],
                protocols: vec!["http".to_string()],
                origin_group: OriginGroup {
                    origins: vec![
                        Origin {
                            host: "origin1.com".to_string(),
                            port: 80,
                            protocol: "http".to_string(),
                        },
                        Origin {
                            host: "origin2.com".to_string(),
                            port: 80,
                            protocol: "http".to_string(),
                        },
                    ],
                },
            },
            route
        );
    }
}
