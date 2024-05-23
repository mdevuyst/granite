//! A dynamically configurable HTTP caching proxy.
//!
use log::info;
use pingora::listeners::TlsSettings;
use pingora::prelude::http_proxy_service;
use pingora::prelude::Opt as CommandLineOptions;
use pingora::server::Server;
use pingora::services::{listening::Service as ListeningService, Service};
use pingora::tls::ssl::SslVerifyMode;
use std::path::Path;
use std::process;
use std::sync::Arc;

mod app_config;
mod cert;
mod config_api;
mod proxy;
mod route_config;
mod route_store;
mod utils;

use crate::app_config::{ApiConfig, AppConfig};
use crate::cert::{cert_provider::CertProvider, cert_store::CertStore};
use crate::config_api::ConfigApi;
use crate::proxy::Proxy;
use crate::route_store::RouteStore;

/// Create and run two services (along with all the necessary dependencies):
/// 1. An HTTP caching proxy service.
/// 2. A config API service that accepts configuration changes (e.g., routes, certificates).
/// Some options are supplied on the command line, and the rest are read from a configuration file.
/// See the user guide for more details on all the available options.
fn main() {
    env_logger::init();

    let opt = CommandLineOptions::default();
    let conf = match opt.conf.as_ref() {
        Some(file) => {
            if !Path::new(file).exists() {
                eprintln!("Config file not found: {file}");
                process::exit(1);
            }
            AppConfig::load_from_yaml(file).unwrap_or_else(|e| {
                eprintln!("Failed to load config file: {file} error: {e}");
                process::exit(1);
            })
        }
        None => AppConfig::default(),
    };

    let mut server = Server::new(Some(opt)).unwrap();
    server.bootstrap();

    let route_store = Arc::new(RouteStore::new());
    let cert_store = Arc::new(CertStore::new());

    let config_api_service = create_config_api(&conf.api, route_store.clone(), cert_store.clone());

    let proxy = Proxy::new(&conf.proxy, &conf.cache, route_store.clone());
    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    for addr in &conf.proxy.http_bind_addrs {
        info!("Adding proxy HTTP listener on {addr}");
        proxy_service.add_tcp(addr);
    }
    for addr in &conf.proxy.https_bind_addrs {
        let cert_provider = CertProvider::new(cert_store.clone());
        let mut tls_settings = TlsSettings::with_callbacks(cert_provider).unwrap();
        tls_settings.enable_h2();
        info!("Adding proxy HTTPS listener on {addr}");
        proxy_service.add_tls_with_settings(addr, None, tls_settings);
    }

    let services: Vec<Box<dyn Service>> = vec![config_api_service, Box::new(proxy_service)];
    server.add_services(services);

    server.run_forever();
}

/// Create a config API service to apply dynamic configuration changes.
/// It can run over HTTP or HTTPS and can also authenticate the caller using mutual TLS, depending
/// on the configuration.
fn create_config_api(
    config: &ApiConfig,
    route_store: Arc<RouteStore>,
    cert_store: Arc<CertStore>,
) -> Box<dyn Service> {
    let config_api = Arc::new(ConfigApi::new(route_store, cert_store));
    let mut config_api_service =
        ListeningService::new("Config API service".to_string(), config_api);

    if config.tls {
        let cert_file = config.cert.as_ref().unwrap();
        let key_file = config.key.as_ref().unwrap();

        let mut tls_settings = TlsSettings::intermediate(cert_file, key_file).unwrap();
        tls_settings.enable_h2();

        if config.mutual_tls {
            let client_cert_file = config.client_cert.as_ref().unwrap();
            tls_settings.set_ca_file(client_cert_file).unwrap();
            tls_settings.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        }

        config_api_service.add_tls_with_settings(config.bind_addr.as_str(), None, tls_settings);
    } else {
        config_api_service.add_tcp(config.bind_addr.as_str());
    }
    info!(
        "Adding Config API on {} TLS: {} mTLS: {}",
        config.bind_addr.as_str(),
        config.tls,
        config.mutual_tls
    );

    Box::new(config_api_service)
}
