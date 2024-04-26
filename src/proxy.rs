use async_trait::async_trait;
use log::info;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use std::sync::Arc;

use crate::route_config::{Origin, Protocol};
use crate::route_store::Route;
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
    https_ports: Vec<u16>,
}

impl Proxy {
    pub fn new(route_store: Arc<RouteStore>, https_ports: &[u16]) -> Proxy {
        Proxy {
            route_store,
            https_ports: https_ports.to_vec(),
        }
    }

    fn find_route(&self, session: &mut Session, ctx: &mut RequestContext) -> Result<()> {
        let host = get_host_header(session)?;
        let path = session.req_header().uri.path();
        let protocol = get_incoming_protocol(session, &self.https_ports)?;
        let route = self
            .route_store
            .get_route(protocol, host, path)
            .ok_or_else(|| Error::explain(HTTPStatus(404), "No route found"))?;

        info!(
            "Matched route '{}' belonging to customer '{}'",
            route.config.name, route.config.customer
        );
        ctx.route = Some(route);

        Ok(())
    }

    fn override_host_header(
        &self,
        upstream_request: &mut RequestHeader,
        ctx: &mut RequestContext,
    ) -> Result<()> {
        let origin = ctx.origin.as_ref().ok_or_else(|| {
            Error::explain(
                HTTPStatus(500),
                "Origin should be set in upstream_request_filter",
            )
        })?;

        if let Some(ref host_header_override) = origin.host_header_override {
            upstream_request.insert_header("host", host_header_override)?;
        }

        Ok(())
    }
}

#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = RequestContext;
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        self.find_route(session, ctx)?;
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let route = ctx
            .route
            .as_ref()
            .ok_or_else(|| Error::explain(HTTPStatus(500), "Missing expected route"))?;

        // TODO: Implement load balancing; don't always pick the first origin.
        let origin = route
            .config
            .origin_group
            .origins
            .first()
            .ok_or_else(|| Error::explain(HTTPStatus(404), "No origins in origin group"))?;

        // TODO: Save a *reference* to the origin in the context.
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
        self.override_host_header(upstream_request, ctx)
    }
}

fn get_host_header(session: &Session) -> Result<&str> {
    // TODO: this doesn't work for HTTP/2.  Maybe search for ":authority" too?
    session
        .get_header(http::header::HOST)
        .ok_or_else(|| Error::explain(HTTPStatus(400), "No host header detected"))?
        .to_str()
        .map_err(|_| Error::explain(HTTPStatus(400), "Non-ascii host header"))
}

pub fn get_incoming_protocol(session: &Session, https_ports: &[u16]) -> Result<Protocol> {
    let server_port = session
        .server_addr()
        .ok_or_else(|| Error::explain(HTTPStatus(500), "No server address"))?
        .as_inet()
        .ok_or_else(|| Error::explain(HTTPStatus(500), "Not an inet socket"))?
        .port();

    match https_ports.contains(&server_port) {
        true => Ok(Protocol::Https),
        false => Ok(Protocol::Http),
    }
}
