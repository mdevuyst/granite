use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// An interface for adding and deleting routes.
pub trait RouteHolder: Send + Sync {
    fn add_route(&self, route: RouteConfig);
    fn delete_route(&self, name: &str);
}

/// The scheme the client used to connect to the proxy.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash)]
pub enum IncomingScheme {
    Http,
    Https,
}

/// The scheme to use for requests to the origin.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Hash, Default)]
pub enum OutgoingScheme {
    /// Always forward requests to the origin using HTTP.
    Http,

    /// Always forward requests to the origin using HTTPS.
    Https,

    /// Match the scheme that the client used.
    #[default]
    MatchIncoming,
}

/// Information about an origin server.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Origin {
    /// The hostname or IP address of the origin server.
    pub host: String,

    /// The port to connect to if the scheme is HTTP.
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// The port to connect to if the scheme is HTTPS.
    #[serde(default = "default_https_port")]
    pub https_port: u16,

    /// An optional host header to send to the origin server.
    pub host_header_override: Option<String>,

    /// An optional SNI to send to the origin server.
    pub sni: Option<String>,

    /// The weight of this origin server.  The higher the weight, the more likely it is to be
    /// selected.  Weights are relative to the weights of other origins in the same group.
    /// E.g., if one origin has a weight of 10 and another has a weight of 20, the second origin is
    /// twice as likely to be selected.
    /// If no weight is specified, the default weight is 10.
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

/// A route configuration.  Route matching is based on the combination of the scheme, host, and path
/// (using longest prefix match).
#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct RouteConfig {
    /// A name for the route.  Must be unique among all routes.
    pub name: String,

    /// The customer this route is for.
    pub customer: String,

    /// The incoming schemes this route matches (HTTP, HTTPS, or both).
    pub incoming_schemes: HashSet<IncomingScheme>,

    /// The hosts this route matches.
    pub hosts: Vec<String>,

    /// The paths this route matches.
    pub paths: Vec<String>,

    /// Whether to enable caching for requests that match this route.
    #[serde(default)]
    pub cache: bool,

    /// The scheme to use for requests to the origin (HTTP, HTTPS, or match the client's scheme).
    #[serde(default)]
    pub outgoing_scheme: OutgoingScheme,

    /// A group of origin servers to select from.
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
