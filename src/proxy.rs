use async_trait::async_trait;
use log::info;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora::Error as PingoraError;
use pingora::ErrorType as PingoraErrorType;
use std::sync::Arc;

use crate::route_store::RouteStore;

pub struct Proxy {
    route_store: Arc<RouteStore>,
}

impl Proxy {
    pub fn new(route_store: Arc<RouteStore>) -> Proxy {
        Proxy { route_store }
    }
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = ();
    fn new_ctx(&self) {}

    async fn upstream_peer(&self, session: &mut Session, _ctx: &mut ()) -> Result<Box<HttpPeer>> {
        // TODO: Use proper error handling.
        let Some(host) = session.get_header("host") else {
            info!("Client made a request with host header");
            return Err(PingoraError::new_down(PingoraErrorType::HTTPStatus(400)));
        };
        let Ok(host) = host.to_str() else {
            info!("Client used non-ascii Host header");
            return Err(PingoraError::new_down(PingoraErrorType::HTTPStatus(400)));
        };
        let Some(route) = self.route_store.get_route(host) else {
            info!("No route found for host: {host}");
            return Err(PingoraError::new_down(PingoraErrorType::HTTPStatus(404)));
        };

        // TODO: Implement load balancing; don't always pick the first origin.
        let Some(origin) = route.origin_group.origins.first() else {
            info!("No origin found for host: {host}");
            return Err(PingoraError::new_down(PingoraErrorType::HTTPStatus(404)));
        };

        // TODO: Implement HTTPS support.
        info!(
            "Routing request to {}:{}",
            origin.host.as_str(),
            origin.port
        );
        Ok(Box::new(HttpPeer::new(
            (origin.host.as_str(), origin.port),
            false,
            "".to_string(),
        )))
    }
}
