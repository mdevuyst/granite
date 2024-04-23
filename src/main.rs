use async_trait::async_trait;
use bytes::Bytes;
use http::{Response, StatusCode};
use pingora::apps::http_app::ServeHttp;
use pingora::prelude::*;
use pingora::protocols::http::ServerSession;
use pingora::services::{listening::Service as ListeningService, Service};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct CustomerConfig {
    host: String,
    origin: String,
}

// TODO: move this to a separate module.
struct ConfigApi {
    route_map: RwLock<HashMap<String, String>>,
}

impl ConfigApi {
    fn new() -> Self {
        ConfigApi {
            route_map: RwLock::new(HashMap::new()),
        }
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

        let request_body: Option<Bytes> = match timeout(
            Duration::from_secs(30),
            http_stream.read_request_body(),
        )
        .await
        {
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

        //let customer_config: serde_json::Result<CustomerConfig> = serde_json::from_slice(&request_body);
        let Ok(customer_config) = serde_json::from_slice::<CustomerConfig>(&request_body) else {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(http::header::CONTENT_TYPE, "text/html")
                .header(http::header::CONTENT_LENGTH, 0)
                .body(Vec::new())
                .unwrap();
        };

        let mut route_map = self.route_map.write().unwrap();
        let _ = (*route_map).insert(customer_config.host, customer_config.origin);

        let response_body = format!("Current config: {:?}\n", *route_map).as_bytes().to_vec();

        Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "text/html")
            .header(http::header::CONTENT_LENGTH, response_body.len())
            .body(response_body)
            .unwrap()
    }
}

fn new_config_api_service(config_api: Arc<ConfigApi>) -> ListeningService<ConfigApi> {
    ListeningService::new("Config API service".to_string(), config_api)
}

fn main() {
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    let config_api = Arc::new(ConfigApi::new());

    let mut config_api_service = new_config_api_service(config_api.clone());
    config_api_service.add_tcp("0.0.0.0:5000");

    // TODO: Implement a proxy service.  The proxy service can take the config_api to get access to the route_map.

    let services: Vec<Box<dyn Service>> = vec![Box::new(config_api_service)];
    my_server.add_services(services);

    my_server.run_forever();
}
