use crate::route_config::{Route, RouteHolder};
use async_trait::async_trait;
use bytes::Bytes;
use http::{Response, StatusCode};
use log::{error, info};
use pingora::apps::http_app::ServeHttp;
use pingora::prelude::*;
use pingora::protocols::http::ServerSession;
use std::sync::Arc;
use std::time::Duration;

pub struct ConfigApi {
    route_holder: Arc<dyn RouteHolder>,
}

impl ConfigApi {
    pub fn new(route_holder: Arc<dyn RouteHolder>) -> Self {
        ConfigApi { route_holder }
    }

    async fn add_route(&self, session: &mut ServerSession) -> Response<Vec<u8>> {
        let method = &session.req_header().as_ref().method;
        if method != http::Method::POST {
            error!("Received unsupported method {method:?} in /route/add");
            return build_response(StatusCode::METHOD_NOT_ALLOWED);
        }

        let request_body: Option<Bytes> =
            match timeout(Duration::from_secs(30), session.read_request_body()).await {
                Ok(res) => match res {
                    Ok(res) => res,
                    Err(_) => None,
                },
                Err(_) => None,
            };

        let Some(request_body) = request_body else {
            info!("Unable to read request body");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "text/html")
                .header(http::header::CONTENT_LENGTH, 0)
                .body(Vec::new())
                .unwrap();
        };

        let Ok(route) = serde_json::from_slice::<Route>(&request_body) else {
            info!("Failed to parse request body as Route");
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "text/html")
                .header(http::header::CONTENT_LENGTH, 0)
                .body(Vec::new())
                .unwrap();
        };

        info!("Adding route for customer: {}", route.customer.as_str());
        self.route_holder.add_route(route);

        let response_body = "Change accepted\n".as_bytes().to_vec();

        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "text/html")
            .header(http::header::CONTENT_LENGTH, response_body.len())
            .body(response_body)
            .unwrap()
    }
}

#[async_trait]
impl ServeHttp for ConfigApi {
    async fn response(&self, http_stream: &mut ServerSession) -> Response<Vec<u8>> {
        let path = http_stream.req_header().uri.path();
        match path {
            "/route/add" => self.add_route(http_stream).await,
            _ => {
                error!("Unhandled path: {path}");
                build_response(StatusCode::NOT_FOUND)
            }
        }
    }
}

fn build_response(status: StatusCode) -> Response<Vec<u8>> {
    Response::builder().status(status).body(Vec::new()).unwrap()
}
