use async_trait::async_trait;
use log::info;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora::Error as PingoraError;
use pingora::ErrorType as PingoraErrorType;
use std::sync::Arc;

use crate::route_config::{Origin, Protocol, Route};
use crate::route_store::RouteStore;

#[derive(Debug)]
pub struct RequestContext {
    route: Option<Arc<Route>>,
    origin: Option<Origin>,
}

impl RequestContext {
    fn new() -> RequestContext {
        RequestContext {
            route: None,
            origin: None,
        }
    }
}

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
    type CTX = RequestContext;
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // TODO: Use proper error handling.
        let Some(host) = session.get_header("host") else {
            info!("Client made a request without a host header");
            session.respond_error(400).await;
            return Ok(true);
        };
        let Ok(host) = host.to_str() else {
            info!("Client used non-ascii Host header");
            session.respond_error(400).await;
            return Ok(true);
        };

        let path = session.req_header().uri.path();

        // TODO: Pass the actual incoming protocol. May need to infer this from the local sockaddr.
        let Some(route) = self.route_store.get_route(Protocol::Http, host, path) else {
            info!("No route found for host: {host}");
            session.respond_error(404).await;
            return Ok(true);
        };

        info!(
            "Matched route '{}' belonging to customer '{}'",
            route.name, route.customer
        );
        ctx.route = Some(route);

        Ok(false)
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let Some(ref route) = ctx.route else {
            return Err(PingoraError::new_in(PingoraErrorType::HTTPStatus(500)));
        };
        // If we've gotten this far, we know that the route exists and the host header is ascii.
        let host = session.get_header("host").unwrap().to_str().unwrap();

        // TODO: Implement load balancing; don't always pick the first origin.
        let Some(origin) = route.origin_group.origins.first() else {
            info!("No origin found for host: {host}");
            return Err(PingoraError::new_down(PingoraErrorType::HTTPStatus(404)));
        };

        // TODO: Save a reference to the origin in the context.
        ctx.origin = Some(origin.clone());

        info!(
            "Routing request to {}:{}",
            origin.host.as_str(),
            origin.port
        );

        let sni = match origin.sni.as_ref() {
            Some(sni) => sni.clone(),
            None => "".to_string(),
        };

        Ok(Box::new(HttpPeer::new(
            (origin.host.as_str(), origin.port),
            origin.protocol == Protocol::Https,
            sni,
        )))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let Some(ref origin) = ctx.origin else {
            return Err(PingoraError::new_in(PingoraErrorType::HTTPStatus(500)));
        };

        if let Some(ref host_header_override) = origin.host_header_override {
            upstream_request.insert_header("Host", host_header_override)?;
        }

        Ok(())
    }
}
