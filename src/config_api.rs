use crate::route_config::{Route, RouteHolder};
use async_trait::async_trait;
use bytes::Bytes;
use http::{Response, StatusCode};
use pingora::apps::http_app::ServeHttp;
use pingora::prelude::*;
use pingora::protocols::http::ServerSession;
use std::sync::Arc;
use std::time::Duration;

pub struct ConfigApi {
    route_holder: Arc<dyn RouteHolder>,
    // route_map: RwLock<HashMap<String, String>>,
}

impl ConfigApi {
    pub fn new(route_holder: Arc<dyn RouteHolder>) -> Self {
        ConfigApi { route_holder }
    }
}

#[async_trait]
impl ServeHttp for ConfigApi {
    async fn response(&self, http_stream: &mut ServerSession) -> Response<Vec<u8>> {
        if http_stream.req_header().as_ref().method != http::Method::POST {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "text/html")
                .header(http::header::CONTENT_LENGTH, 0)
                .body(Vec::new())
                .unwrap();
        }

        let request_body: Option<Bytes> =
            match timeout(Duration::from_secs(30), http_stream.read_request_body()).await {
                Ok(res) => match res {
                    Ok(res) => res,
                    Err(_) => None,
                },
                Err(_) => None,
            };

        let Some(request_body) = request_body else {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "text/html")
                .header(http::header::CONTENT_LENGTH, 0)
                .body(Vec::new())
                .unwrap();
        };

        let Ok(route) = serde_json::from_slice::<Route>(&request_body) else {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "text/html")
                .header(http::header::CONTENT_LENGTH, 0)
                .body(Vec::new())
                .unwrap();
        };

        self.route_holder.add_route(route);

        let response_body = "Thanks\n".as_bytes().to_vec();

        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "text/html")
            .header(http::header::CONTENT_LENGTH, response_body.len())
            .body(response_body)
            .unwrap()
    }
}
