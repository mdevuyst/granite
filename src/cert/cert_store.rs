use crate::cert::cert_config::CertHolder;
use pingora::tls::pkey::{PKey, Private};
use pingora::tls::x509::X509;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use log::warn;

pub type CertAndKey = Arc<(X509, PKey<Private>)>;

struct InnerStore {
    host_to_cert: HashMap<String, CertAndKey>,
}

impl InnerStore {
    fn new() -> Self {
        InnerStore {
            host_to_cert: HashMap::new(),
        }
    }
}

pub struct CertStore {
    inner: RwLock<InnerStore>,
}

impl CertStore {
    pub fn new() -> Self {
        CertStore {
            inner: RwLock::new(InnerStore::new()),
        }
    }

    pub fn get_cert(&self, host: &str) -> Option<CertAndKey> {
        let inner = self.inner.read().unwrap();

        let cert_and_key = inner.host_to_cert.get(host)?;
        Some(cert_and_key.clone())
    }
}

impl CertHolder for CertStore {
    fn add_cert(&self, host: &str, cert: X509, key: PKey<Private>) {
        let mut inner = self.inner.write().unwrap();

        inner
            .host_to_cert
            .insert(host.to_string(), Arc::new((cert, key)));
    }

    fn delete_cert(&self, host: &str) {
        let mut inner = self.inner.write().unwrap();

        let cert_and_key = inner.host_to_cert.remove(host);

        if cert_and_key.is_some() {
            warn!("Attempted to delete a cert that doesn't exis host={host}");
        }
    }
}
