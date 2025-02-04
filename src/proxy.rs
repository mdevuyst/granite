//! The caching proxy.

use async_trait::async_trait;
use log::{info, warn};
use once_cell::sync::{Lazy, OnceCell};
use pingora::cache::{
    cache_control::CacheControl, eviction::simple_lru, filters::resp_cacheable, lock::CacheLock,
    CacheMetaDefaults, CachePhase, MemCache, NoCacheReason, RespCacheable,
};
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use pingora::proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use rand::distributions::{Distribution, WeightedIndex};
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::lookup_host;

use crate::app_config::{CacheConfig, ProxyConfig};
use crate::route_config::{IncomingScheme, Origin, OutgoingScheme};
use crate::route_store::Route;
use crate::route_store::RouteStore;
use crate::utils;

static CACHE_BACKEND: Lazy<MemCache> = Lazy::new(MemCache::new);
/// By default, cache all responses for 5 minutes.  This can be overridden by the origin's cache
/// control headers.
const CACHE_META_DEFAULTS: CacheMetaDefaults = CacheMetaDefaults::new(|_| Some(300), 1, 1);
static EVICTION_MANAGER: OnceCell<simple_lru::Manager> = OnceCell::new();
static CACHE_LOCK: Lazy<CacheLock> =
    Lazy::new(|| CacheLock::new(std::time::Duration::from_secs(2)));

/// A context that is available throughout the lifecycle of a request.
#[derive(Debug)]
pub struct RequestContext {
    /// The route that was matched for the request.
    route: Option<Arc<Route>>,
    /// The origin that was selected for the request.
    origin: Option<Origin>,
    /// The index of the origin that was selected for the request.
    origin_index: Option<usize>,
    /// The number of attempts to connect to an origin.
    tries: u16,
}

impl RequestContext {
    fn new() -> RequestContext {
        RequestContext {
            route: None,
            origin: None,
            origin_index: None,
            tries: 0,
        }
    }
}

pub struct Proxy {
    /// A means to look up routes.
    route_store: Arc<RouteStore>,

    /// The ports that are used for HTTPS.
    https_ports: Vec<u16>,

    /// The amount of time (in seconds) an origin is marked down if it fails to connect.
    origin_down_time: u64,

    /// The maximum number of times to retry connecting to an origin.
    connection_retry_limit: u16,
}

impl Proxy {
    pub fn new(
        proxy_config: &ProxyConfig,
        cache_config: &CacheConfig,
        route_store: Arc<RouteStore>,
    ) -> Proxy {
        let https_ports = utils::collect_ports(&proxy_config.https_bind_addrs);

        let eviction_manager = simple_lru::Manager::new(cache_config.max_size);
        if EVICTION_MANAGER.set(eviction_manager).is_err() {
            warn!("Eviction manager has already been initialized");
        }

        Proxy {
            route_store,
            https_ports,
            origin_down_time: proxy_config.origin_down_time,
            connection_retry_limit: proxy_config.connection_retry_limit,
        }
    }

    /// Find the route that matches the request.
    /// The scheme and host header must match a route's scheme and host exactly.  The path is a
    /// longest-prefix match.
    /// If a matching route is found, it is stored in the context.  Else, a 404 error is returned.
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

    /// Override the host header in the upstream request if the origin configuration has a host
    /// header override.
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

    /// Pick an origin from the origin group of the route using a weighted random selection.
    /// Origins marked down are not eligible for selection.
    /// Return the index within the origin group of the selected origin or an error.
    fn select_origin(&self, route: &Arc<Route>) -> Result<usize> {
        let origins = &route.config.origin_group.origins;
        if origins.is_empty() {
            return Error::e_explain(HTTPStatus(502), "No origins in origin group");
        }

        let mut down_origins: Vec<usize> = Vec::new();

        {
            // If any origins were marked down more than N seconds ago, unmark them.
            // First, take a read lock and check if any were marked down more than N seconds ago.
            // Most of the time, we shouldn't find any that need to be unmarked.
            let mut found_expired = false;
            {
                let state = route.state.read().unwrap();
                for (_, &timestamp) in state.down_endpoints.iter() {
                    if timestamp.elapsed() > Duration::from_secs(self.origin_down_time) {
                        found_expired = true;
                        break;
                    }
                }
            }
            // In the rare chance that any were found, take a write lock and remove them.
            if found_expired {
                info!(
                    "Unmarking origin(s) that were marked down more than {} seconds ago",
                    self.origin_down_time
                );
                let mut state = route.state.write().unwrap();
                state
                    .down_endpoints
                    .retain(|_, v| v.elapsed() <= Duration::from_secs(self.origin_down_time));
            }

            // Copy the list of origins still marked down.
            let state = route.state.read().unwrap();
            for (&index, _) in state.down_endpoints.iter() {
                down_origins.push(index);
            }
        }

        // Get a list of eligible origins along with their weights.  The list of eligible origins includes
        // all the origins that aren't marked down; Or, if all origins are marked down, then all are eligible.
        // The data structure is a vector of tuples, where the first element is the index of the origin in the
        // origin group and the second element is the weight of the origin.
        let mut eligible_origins_and_weights: Vec<(usize, u16)> = Vec::new();
        if down_origins.len() == origins.len() {
            info!("All origins marked down. Picking a down origin");
            for (index, origin) in origins.iter().enumerate() {
                eligible_origins_and_weights.push((index, origin.weight));
            }
        } else {
            for (index, origin) in origins.iter().enumerate() {
                if !down_origins.contains(&index) {
                    eligible_origins_and_weights.push((index, origin.weight));
                }
            }
        }

        // Select an eligible origin randomly using the weights of all eligible origins.
        let mut rng = rand::thread_rng();
        let weights: Vec<_> = eligible_origins_and_weights.iter().map(|e| e.1).collect();
        let dist = WeightedIndex::new(weights)
            .or_else(|e| Error::e_because(HTTPStatus(500), "Unable to create WeightedIndex", e))?;
        let index_into_eligible_origins = dist.sample(&mut rng);
        Ok(eligible_origins_and_weights[index_into_eligible_origins].0)
    }

    fn mark_origin_down(route: &Route, origin_index: usize) -> Result<()> {
        let mut state = route.state.write().unwrap();
        let origins = &route.config.origin_group.origins;
        if origins.is_empty() {
            return Err(Error::new_str("No origins in origin group"));
        }
        if let Entry::Vacant(e) = state.down_endpoints.entry(origin_index) {
            info!("Marking origin '{}' down", &origins[origin_index].host);
            let _ = e.insert(Instant::now());
        }
        Ok(())
    }
}

/// The implementation of the interface between Pingora and the proxy.
#[async_trait]
impl ProxyHttp for Proxy {
    type CTX = RequestContext;
    fn new_ctx(&self) -> Self::CTX {
        RequestContext::new()
    }

    /// The first phase in the request lifetime.  This is where we try to find a matching route
    /// which will be saved in the request context.
    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        self.find_route(session, ctx)?;
        Ok(false)
    }

    /// Select an origin to forward the request to.
    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let route = ctx
            .route
            .as_ref()
            .ok_or_else(|| Error::explain(HTTPStatus(500), "Missing expected route"))?;

        let origin_index = self.select_origin(route)?;
        let origin = &route.config.origin_group.origins[origin_index];

        // TODO: Save a *reference* to the origin in the context.
        ctx.origin = Some(origin.clone());
        ctx.origin_index = Some(origin_index);

        // Determine whether to connect to the origin using TLS, what port to use, what SNI to use
        // based on the origin's configuration.
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
        let sni = match origin.sni.as_ref() {
            Some(sni) => sni.clone(),
            None => "".to_string(),
        };

        info!(
            "Routing request to {}:{}",
            origin.host.as_str(),
            outgoing_port
        );

        ctx.tries += 1;

        // Resolve the host to an IP address (asynchronously).
        // Note: `HttpPeer::new` can also do this, but it is blocking.
        let addr = match lookup_host((origin.host.as_str(), outgoing_port)).await {
            // For now, we only use the first address found.
            Ok(mut addrs) => addrs
                .next()
                .ok_or_else(|| Error::explain(HTTPStatus(502), "No address found"))?,
            Err(e) => {
                // Mark the origin down and return an error.  If the connection attempt should be
                // retried, Pingora will call `upstream_peer` again
                Self::mark_origin_down(route, origin_index).expect("Expect at least one origin");
                let mut e = Error::because(HTTPStatus(502), "Unable to resolve host", e);
                if ctx.tries <= self.connection_retry_limit {
                    e.set_retry(true);
                }
                return Err(e);
            }
        };

        let mut peer = Box::new(HttpPeer::new(addr, use_tls, sni));

        // If using HTTP/2, try HTTP/2 but fall back to HTTP/1.1 if it fails.
        if use_tls {
            peer.options.set_http_version(2, 1);
        }

        Ok(peer)
    }

    /// Determine if caching is enabled for this request based on the route configuration.
    /// Calls `session.cache.enable()` to enable caching.
    fn request_cache_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<()> {
        let Some(route) = &ctx.route else {
            return Ok(());
        };
        if !route.config.cache {
            return Ok(());
        }

        session.cache.enable(
            &*CACHE_BACKEND,
            Some(EVICTION_MANAGER.get().unwrap()),
            None,
            Some(&*CACHE_LOCK),
        );
        Ok(())
    }

    /// Modify the request headers before sending them to the upstream server.
    /// Override the host header in the upstream request if the origin configuration has a host
    /// header override.
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        self.override_host_header(upstream_request, ctx)
    }

    /// Handle the case where the connection to the upstream server fails.
    /// Mark the origin down for a while and specify whether the connection attempt should be
    /// retried (possibly to a different origin).
    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        mut e: Box<Error>,
    ) -> Box<Error> {
        let Some(route) = ctx.route.as_ref() else {
            return e;
        };
        let origins = &route.config.origin_group.origins;
        if origins.is_empty() {
            return e;
        }
        let Some(origin_index) = ctx.origin_index else {
            return e;
        };

        if Self::mark_origin_down(route, origin_index).is_err() {
            return e;
        }

        // Retry once.
        if ctx.tries > self.connection_retry_limit {
            info!("Connection retry limit exceed");
            return e;
        }
        info!("Retrying connection");
        e.set_retry(true);
        e
    }

    /// Determine if the response should be cached based on the response headers.
    /// This function is only called if caching was enabled in `request_cache_filter`.
    fn response_cache_filter(
        &self,
        _session: &Session,
        resp: &ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<RespCacheable> {
        let cc = CacheControl::from_resp_headers(resp);
        Ok(resp_cacheable(
            cc.as_ref(),
            resp,
            false,
            &CACHE_META_DEFAULTS,
        ))
    }

    /// Modify the response headers before sending them to the client.
    /// Insert a header indicating the cache status of the response.
    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        let cache_status = if session.cache.enabled() {
            match session.cache.phase() {
                CachePhase::Hit => "hit",
                CachePhase::Miss => "miss",
                CachePhase::Stale => "stale",
                CachePhase::Expired => "expired",
                CachePhase::Revalidated | CachePhase::RevalidatedNoCache(_) => "revalidated",
                _ => "invalid",
            }
        } else {
            match session.cache.phase() {
                CachePhase::Disabled(NoCacheReason::Deferred) => "deferred",
                _ => "no-cache",
            }
        };

        info!("Cache status: {}", cache_status);
        upstream_response.insert_header("x-cache-status", cache_status)?;
        Ok(())
    }
}

/// Get the host header from the request.  If HTTP/2 or a missing host header, use the "authority"
/// header or portion of the URI instead.
/// Return a 400 status code if no header could be found.
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

/// Infer the scheme of the incoming request based on the server port (because Pingora doesn't
/// directly provide the scheme).
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
