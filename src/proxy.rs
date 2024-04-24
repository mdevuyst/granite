use async_trait::async_trait;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::route_config::{Route, RouteHolder};

pub struct Proxy {
    host_to_route: RwLock<HashMap<String, Route>>,
}

impl Proxy {
    pub fn new() -> Proxy {
        Proxy {
            host_to_route: RwLock::new(HashMap::new()),
        }
    }
}

impl RouteHolder for Proxy {
    // TODO: Don't clone the route. Save it and have multiple hash table entries point to it.
    fn add_route(&mut self, route: Route) {
        let mut host_to_route = self.host_to_route.write().unwrap();
        for host in &route.hosts {
            (*host_to_route).insert(host.to_string(), route.clone());
        }
    }
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = ();
    fn new_ctx(&self) {}

    async fn upstream_peer(&self, _session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
        Ok(Box::new(HttpPeer::new(
            ("1.0.0.1", 443),
            true,
            "one.one.one.one".to_string(),
        )))
    }
}
