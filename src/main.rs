use async_trait::async_trait;
use http::{Response, StatusCode};
use pingora::apps::http_app::ServeHttp;
use pingora::prelude::*;
use pingora::protocols::http::ServerSession;
use pingora::services::{listening::Service as ListeningService, Service};
use std::sync::Arc;

// TODO: move this to a separate module.
struct ConfigApi;

#[async_trait]
impl ServeHttp for ConfigApi {
    async fn response(&self, http_stream: &mut ServerSession) -> Response<Vec<u8>> {
        // TODO: Read the request as a config update, validate it, apply it, and return a response.
        let body = "Hello, World!\n".as_bytes().to_vec();
        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "text/html")
            .header(http::header::CONTENT_LENGTH, body.len())
            .body(body)
            .unwrap()
    }
}

fn new_config_api() -> ListeningService<ConfigApi> {
    ListeningService::new("Config API service".to_string(), Arc::new(ConfigApi {}))
}

fn main() {
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    let mut config_api_service = new_config_api();
    config_api_service.add_tcp("0.0.0.0:5000");

    // TODO: Implement a proxy service.

    let services: Vec<Box<dyn Service>> = vec![Box::new(config_api_service)];
    my_server.add_services(services);

    my_server.run_forever();
}
