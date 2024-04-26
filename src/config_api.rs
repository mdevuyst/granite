use crate::route_config::{Route, RouteHolder};
use async_trait::async_trait;
use http::{Response, StatusCode};
use log::{error, info};
use pingora::apps::http_app::ServeHttp;
use pingora::protocols::http::ServerSession;
use std::sync::Arc;

pub struct ConfigApi {
    route_holder: Arc<dyn RouteHolder>,
}

#[async_trait]
impl ServeHttp for ConfigApi {
    async fn response(&self, http_stream: &mut ServerSession) -> Response<Vec<u8>> {
        let path = http_stream.req_header().uri.path();
        match path {
            "/route/add" => self.add_route(http_stream).await,
            "/route/delete" => self.delete_route(http_stream).await,
            _ => {
                error!("Unhandled path: {path}");
                build_response(StatusCode::NOT_FOUND, "")
            }
        }
    }
}

impl ConfigApi {
    pub fn new(route_holder: Arc<dyn RouteHolder>) -> Self {
        ConfigApi { route_holder }
    }

    async fn add_route(&self, session: &mut ServerSession) -> Response<Vec<u8>> {
        let method = &session.req_header().as_ref().method;
        if method != http::Method::POST {
            error!("Received unsupported method {method:?}");
            return build_response(StatusCode::METHOD_NOT_ALLOWED, "");
        }

        let request_body = session.read_request_body().await.ok().flatten();
        let Some(request_body) = request_body else {
            error!("Unable to read request body");
            return build_response(StatusCode::BAD_REQUEST, "");
        };

        let route = serde_json::from_slice::<Route>(&request_body);
        let Ok(route) = route else {
            error!("Failed to parse request body as Route");
            return build_response(StatusCode::BAD_REQUEST, "");
        };

        info!(
            "Adding route {} for customer: {}",
            &route.name, &route.customer
        );
        self.route_holder.add_route(route);

        build_response(StatusCode::OK, "Success\n")
    }

    async fn delete_route(&self, session: &mut ServerSession) -> Response<Vec<u8>> {
        let method = &session.req_header().as_ref().method;
        if method != http::Method::POST {
            error!("Received unsupported method {method:?}");
            return build_response(StatusCode::METHOD_NOT_ALLOWED, "");
        }

        let request_body = session.read_request_body().await.ok().flatten();
        let Some(request_body) = request_body else {
            error!("Unable to read request body");
            return build_response(StatusCode::BAD_REQUEST, "");
        };

        let Ok(route_name) = String::from_utf8(request_body.to_vec()) else {
            error!("route name not UTF-8");
            return build_response(StatusCode::BAD_REQUEST, "");
        };

        info!("Deleting route {}", &route_name);
        self.route_holder.delete_route(&route_name);

        build_response(StatusCode::OK, "Success\n")
    }
}

fn build_response(status: StatusCode, body: &str) -> Response<Vec<u8>> {
    let body = body.as_bytes().to_vec();
    Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "text/html")
        .header(http::header::CONTENT_LENGTH, body.len())
        .body(body)
        .unwrap()
}
