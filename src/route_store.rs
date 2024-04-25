use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use log::info;

use crate::route_config::{Protocol, Route, RouteHolder};

struct InnerStore {
    http_host_to_route: HashMap<String, Vec<Arc<Route>>>,
    https_host_to_route: HashMap<String, Vec<Arc<Route>>>,
    name_to_route: HashMap<String, Arc<Route>>,
}

impl InnerStore {
    fn new() -> Self {
        InnerStore {
            http_host_to_route: HashMap::new(),
            https_host_to_route: HashMap::new(),
            name_to_route: HashMap::new(),
        }
    }
}

pub struct RouteStore {
    inner: RwLock<InnerStore>,
}

impl RouteStore {
    pub fn new() -> Self {
        RouteStore {
            inner: RwLock::new(InnerStore::new()),
        }
    }

    pub fn get_route(&self, protocol: Protocol, host: &str, path: &str) -> Option<Arc<Route>> {
        let inner = self.inner.read().unwrap();

        // Look up the routes for the given host.
        let host_to_route = match protocol {
            Protocol::Http => &inner.http_host_to_route,
            Protocol::Https => &inner.https_host_to_route,
        };
        let routes = host_to_route.get(host)?;
        if routes.is_empty() {
            return None;
        }
        info!("Found {} routes for host: {}", routes.len(), host);

        // Find the route with the longest matching path.
        let mut longest_path_length = 0;
        let mut best_match_route: Option<Arc<Route>> = None;
        for route in routes {
            info!("Checking route {}", route.name);
            for candidate_path in &route.paths {
                info!("Checking if {} starts with {}", path, &candidate_path);
                if path.starts_with(candidate_path) && candidate_path.len() > longest_path_length {
                    longest_path_length = candidate_path.len();
                    best_match_route = Some(route.clone());
                }
            }
        }

        best_match_route
    }
}

impl RouteHolder for RouteStore {
    fn add_route(&self, route: Route) {
        let route = Arc::new(route);
        let mut inner = self.inner.write().unwrap();
        inner
            .name_to_route
            .insert(route.name.clone(), route.clone());
        for protocol in route.inbound_protocols.iter() {
            let host_to_route = match protocol {
                Protocol::Http => &mut inner.http_host_to_route,
                Protocol::Https => &mut inner.https_host_to_route,
            };
            for host in &route.hosts {
                host_to_route
                    .entry(host.to_string())
                    .or_insert_with(Vec::new)
                    .push(route.clone());
            }
        }
    }
}
