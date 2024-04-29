use std::sync::RwLock;
use std::time::Instant;
use std::{collections::HashMap, sync::Arc};

use log::{debug, warn};

use crate::route_config::{IncomingScheme, RouteConfig, RouteHolder};

#[derive(Debug, Default)]
pub struct RouteState {
    // TODO: Utilize this struct for route state.
    pub down_endpoints: HashMap<usize, Instant>, // Key: index of down origin, Value: time it was marked down.
}

#[derive(Debug, Default)]
pub struct Route {
    pub config: RouteConfig,
    pub state: RwLock<RouteState>,
}

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

    pub fn get_route(
        &self,
        protocol: IncomingScheme,
        host: &str,
        path: &str,
    ) -> Option<Arc<Route>> {
        let inner = self.inner.read().unwrap();

        // Look up the routes for the given host.
        let host_to_route = match protocol {
            IncomingScheme::Http => &inner.http_host_to_route,
            IncomingScheme::Https => &inner.https_host_to_route,
        };
        let routes = host_to_route.get(host)?;
        if routes.is_empty() {
            return None;
        }
        debug!("Found {} routes for host: {}", routes.len(), host);

        // Find the route with the longest matching path.
        let mut longest_path_length = 0;
        let mut best_match_route: Option<Arc<Route>> = None;
        for route in routes {
            for candidate_path in &route.config.paths {
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
    fn add_route(&self, route_config: RouteConfig) {
        let mut inner = self.inner.write().unwrap();

        // If a route with the same name already exists, delete it first.
        let name = route_config.name.as_str();
        if let Some(route) = inner.name_to_route.get(name) {
            let route = route.clone();

            for protocol in route.config.incoming_schemes.iter() {
                let host_to_route = match protocol {
                    IncomingScheme::Http => &mut inner.http_host_to_route,
                    IncomingScheme::Https => &mut inner.https_host_to_route,
                };
                for host in &route.config.hosts {
                    let routes = host_to_route
                        .get_mut(host)
                        .unwrap_or_else(|| panic!("No routes for {host}. Expected {name}"));
                    let position = routes
                        .iter()
                        .position(|r| r.config.name == name)
                        .unwrap_or_else(|| panic!("Route {name} not found for host {host}"));
                    let _ = routes.remove(position);
                    if routes.is_empty() {
                        let _ = host_to_route.remove(host);
                    }
                }
            }

            let _ = inner.name_to_route.remove(name);
        }

        // Add the new route while still under the lock (this is important so that no reader
        // experiences a lookup miss while a route is being changed).
        let route = Arc::new(Route {
            config: route_config,
            state: RwLock::new(RouteState::default()),
        });

        inner
            .name_to_route
            .insert(route.config.name.clone(), route.clone());

        for protocol in route.config.incoming_schemes.iter() {
            let host_to_route = match protocol {
                IncomingScheme::Http => &mut inner.http_host_to_route,
                IncomingScheme::Https => &mut inner.https_host_to_route,
            };
            for host in &route.config.hosts {
                host_to_route
                    .entry(host.to_string())
                    .or_insert_with(Vec::new)
                    .push(route.clone());
            }
        }
    }

    fn delete_route(&self, name: &str) {
        let mut inner = self.inner.write().unwrap();

        let Some(route) = inner.name_to_route.get(name) else {
            warn!("Attempted to delete a route that doesn't exis name={name}");
            return;
        };
        let route = route.clone();

        for protocol in route.config.incoming_schemes.iter() {
            let host_to_route = match protocol {
                IncomingScheme::Http => &mut inner.http_host_to_route,
                IncomingScheme::Https => &mut inner.https_host_to_route,
            };
            for host in &route.config.hosts {
                let routes = host_to_route
                    .get_mut(host)
                    .unwrap_or_else(|| panic!("No routes for {host}. Expected {name}"));
                let position = routes
                    .iter()
                    .position(|r| r.config.name == name)
                    .unwrap_or_else(|| panic!("Route {name} not found for host {host}"));
                let _ = routes.remove(position);
                if routes.is_empty() {
                    let _ = host_to_route.remove(host);
                }
            }
        }

        let _ = inner.name_to_route.remove(name);
    }
}
