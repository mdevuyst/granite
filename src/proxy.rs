use async_trait::async_trait;
use log::info;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use rand::distributions::{Distribution, WeightedIndex};
use std::sync::Arc;

use crate::route_config::{IncomingScheme, Origin, OutgoingScheme};
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
        let protocol = get_incoming_scheme(session, &self.https_ports)?;
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
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let route = ctx
            .route
            .as_ref()
            .ok_or_else(|| Error::explain(HTTPStatus(500), "Missing expected route"))?;

        let origins = &route.config.origin_group.origins;
        if origins.is_empty() {
            return Error::e_explain(HTTPStatus(404), "No origins in origin group");
        }
        let mut rng = rand::thread_rng();
        let weights: Vec<_> = origins.iter().map(|e| e.weight).collect();
        let dist = WeightedIndex::new(weights)
            .or_else(|e| Error::e_because(HTTPStatus(500), "Unable to create WeightedIndex", e))?;
        let index = dist.sample(&mut rng);
        let origin = &origins[index];

        // TODO: Save a *reference* to the origin in the context.
        ctx.origin = Some(origin.clone());

        let incoming_scheme = get_incoming_scheme(session, &self.https_ports)?;
        let use_tls = match &route.config.outgoing_scheme {
            OutgoingScheme::Http => false,
            OutgoingScheme::Https => true,
            OutgoingScheme::MatchIncoming => match &incoming_scheme {
                IncomingScheme::Http => false,
                IncomingScheme::Https => true,
            },
        };
        let outgoing_port = if use_tls {
            origin.https_port
        } else {
            origin.http_port
        };

        info!(
            "Routing request to {}:{}",
            origin.host.as_str(),
            outgoing_port
        );

        let sni = match origin.sni.as_ref() {
            Some(sni) => sni.clone(),
            None => "".to_string(),
        };

        Ok(Box::new(HttpPeer::new(
            (origin.host.as_str(), outgoing_port),
            use_tls,
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
    let host = match session.get_header(http::header::HOST) {
        Some(host_header) => host_header
            .to_str()
            .map_err(|_| Error::explain(HTTPStatus(400), "Non-ascii host header")),
        // For HTTP/2, a host header may not be present; check the "authority" instead.
        None => match session.req_header().uri.authority() {
            Some(authority) => Ok(authority.as_str()),
            None => Error::e_explain(HTTPStatus(400), "No host header or authority detected"),
        },
    };

    // If the host contains a colon (e.g., "example.com:443"), return the part before the colon.
    if let Ok(host) = host {
        if let Some(index) = host.find(':') {
            return Ok(&host[..index]);
        }
    }

    host
}

pub fn get_incoming_scheme(session: &Session, https_ports: &[u16]) -> Result<IncomingScheme> {
    let server_port = session
        .server_addr()
        .ok_or_else(|| Error::explain(HTTPStatus(500), "No server address"))?
        .as_inet()
        .ok_or_else(|| Error::explain(HTTPStatus(500), "Not an inet socket"))?
        .port();

    match https_ports.contains(&server_port) {
        true => Ok(IncomingScheme::Https),
        false => Ok(IncomingScheme::Http),
    }
}
