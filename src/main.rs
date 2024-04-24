use pingora::prelude::*;
use pingora::services::{listening::Service as ListeningService, Service};
use std::sync::Arc;

mod config_api;
mod proxy;
mod route_config;

use config_api::ConfigApi;
use proxy::Proxy;

fn main() {
    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    let proxy = Arc::new(Proxy::new());

    let config_api = Arc::new(ConfigApi::new(proxy.clone()));

    let mut config_api_service =
        ListeningService::new("Config API service".to_string(), config_api.clone());

    config_api_service.add_tcp("0.0.0.0:5000");

    // TODO: Implement a proxy service.  The proxy service can take the config_api to get access to the route_map.

    let services: Vec<Box<dyn Service>> = vec![Box::new(config_api_service)];
    my_server.add_services(services);

    my_server.run_forever();
}
