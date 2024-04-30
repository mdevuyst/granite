use cert_provider::CertProvider;
use pingora::listeners::TlsSettings;
use pingora::prelude::*;
use pingora::services::{listening::Service as ListeningService, Service};
use pingora::tls::ssl::SslVerifyMode;
use std::sync::Arc;

mod cert_config;
mod cert_provider;
mod cert_store;
mod config_api;
mod proxy;
mod route_config;
mod route_store;
mod utils;

use cert_store::CertStore;
use config_api::ConfigApi;
use proxy::Proxy;
use route_store::RouteStore;

fn main() {
    env_logger::init();

    // TODO: Parse command-line arguments and optionally load configuration from a file.
    // Include port numbers for the Config API and HTTP proxy services.
    // Include default certs for HTTPS.
    // Add the option to preload routes from a set of files.
    // Pass some settings through to Pingora (like daemonization, logging, etc.).

    let proxy_http_bind_addrs = vec!["0.0.0.0:8080".to_string()];
    let proxy_https_bind_addr = vec!["0.0.0.0:4433".to_string()];

    let config_api_bind_addr = "0.0.0.0:5000";

    // set config_api_cert_and_key_files to None to disable TLS for the Config API.
    // let config_api_cert_and_key_files = Some(("api.crt".to_string(), "api.key".to_string()));
    let config_api_cert_and_key_files: Option<(String, String)> = None;

    // Set config_api_client_cert_file to None to disable client certificate verification.
    // let config_api_client_cert_file = Some("client.crt".to_string());
    let config_api_client_cert_file: Option<String> = None;

    let max_cache_size: usize = 100 * 1024 * 1024; // 100 MB

    let mut server = Server::new(None).unwrap();
    server.bootstrap();

    let route_store = Arc::new(RouteStore::new());
    let cert_store = Arc::new(CertStore::new());

    let config_api = Arc::new(ConfigApi::new(route_store.clone(), cert_store.clone()));
    let mut config_api_service =
        ListeningService::new("Config API service".to_string(), config_api.clone());

    if let Some((cert_file, key_file)) = config_api_cert_and_key_files.as_ref() {
        let mut tls_settings = TlsSettings::intermediate(cert_file, key_file).unwrap();
        tls_settings.enable_h2();

        if let Some(client_cert_file) = config_api_client_cert_file {
            tls_settings.set_ca_file(&client_cert_file).unwrap();
            tls_settings.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        }
        config_api_service.add_tls_with_settings(config_api_bind_addr, None, tls_settings);
    } else {
        config_api_service.add_tcp(config_api_bind_addr);
    }

    let https_ports = utils::collect_ports(&proxy_https_bind_addr);
    let proxy = Proxy::new(route_store.clone(), &https_ports, max_cache_size);
    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    for addr in proxy_http_bind_addrs {
        proxy_service.add_tcp(&addr);
    }
    for addr in proxy_https_bind_addr {
        let cert_provider = CertProvider::new(cert_store.clone());
        let mut tls_settings = TlsSettings::with_callbacks(cert_provider).unwrap();
        tls_settings.enable_h2();
        proxy_service.add_tls_with_settings(&addr, None, tls_settings);
    }

    let services: Vec<Box<dyn Service>> =
        vec![Box::new(config_api_service), Box::new(proxy_service)];
    server.add_services(services);

    server.run_forever();
}
