use async_trait::async_trait;
use log::error;
use pingora::listeners::TlsAccept;
use pingora::tls::ssl::{NameType, SslRef};
use std::sync::Arc;

use crate::cert::cert_store::CertStore;

/// Implementation of the interface with Pingora to provide certificates for TLS connections.
/// It uses a CertStore to look up certificates based on the SNI in the Client Hello.
pub struct CertProvider {
    cert_store: Arc<CertStore>,
}

impl CertProvider {
    pub fn new(cert_store: Arc<CertStore>) -> Box<CertProvider> {
        Box::new(CertProvider { cert_store })
    }
}

#[async_trait]
impl TlsAccept for CertProvider {
    /// Function that Pingora calls during the TLS handshake to provide the certificate and
    /// private key.
    async fn certificate_callback(&self, ssl: &mut SslRef) {
        let Some(sni) = ssl.servername(NameType::HOST_NAME) else {
            error!("Unable to extract SNI from CLIENT HELLO");
            return;
        };
        let sni = sni.to_string();

        let Some(cert_and_key) = self.cert_store.get_cert(&sni) else {
            error!("No cert found for {sni}");
            return;
        };

        let cert = &cert_and_key.0;
        let key = &cert_and_key.1;

        use pingora::tls::ext;
        if ext::ssl_use_certificate(ssl, cert).is_err() {
            error!("Error settings cert for {}", &sni);
            return;
        }
        if ext::ssl_use_private_key(ssl, key).is_err() {
            error!("Error settings private key for {}", &sni);
            return;
        }
    }
}
