use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use crate::route_config::{Route, RouteHolder};

pub struct RouteStore {
    host_to_route: RwLock<HashMap<String, Arc<Route>>>,
}

impl RouteStore {
    pub fn new() -> Self {
        RouteStore {
            host_to_route: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_route(&self, host: &str) -> Option<Arc<Route>> {
        let host_to_route = self.host_to_route.read().unwrap();
        (*host_to_route).get(host).cloned()
    }
}

impl RouteHolder for RouteStore {
    fn add_route(&self, route: Route) {
        let route = Arc::new(route);
        let mut host_to_route = self.host_to_route.write().unwrap();
        for host in &route.hosts {
            (*host_to_route).insert(host.to_string(), route.clone());
        }
    }
}
