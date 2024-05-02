use log::warn;
use pingora::tls::pkey::{PKey, Private};
use pingora::tls::x509::X509;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use crate::cert::cert_config::CertHolder;

pub type CertAndKey = Arc<(X509, PKey<Private>)>;

/// A store of certificates and keys, indexed by hostname/SNI.
pub struct CertStore {
    // Protect the internal data structure(s) that enables fast route lookups, additions,
    // and deletions with a read-writer lock.  Reads are frequent (for every TLS connection),
    // but writes are infrequent (only when the config API service is used to update a cert binding).
    inner: RwLock<InnerStore>,
}

/// The inner protected part of the CertStore.
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

impl CertStore {
    pub fn new() -> Self {
        CertStore {
            inner: RwLock::new(InnerStore::new()),
        }
    }

    /// Find a certificate and key pair for the given hostname/SNI.
    pub fn get_cert(&self, host: &str) -> Option<CertAndKey> {
        let inner = self.inner.read().unwrap();

        let cert_and_key = inner.host_to_cert.get(host)?;
        Some(cert_and_key.clone())
    }
}

impl CertHolder for CertStore {
    /// Add a certificate binding (hostname/SNI, certificate, and key).
    fn add_cert(&self, host: &str, cert: X509, key: PKey<Private>) {
        let mut inner = self.inner.write().unwrap();

        inner
            .host_to_cert
            .insert(host.to_string(), Arc::new((cert, key)));
    }

    /// Delete a certificate binding for the given hostname/SNI.
    fn delete_cert(&self, host: &str) {
        let mut inner = self.inner.write().unwrap();

        let cert_and_key = inner.host_to_cert.remove(host);

        if cert_and_key.is_some() {
            warn!("Attempted to delete a cert that doesn't exist host={host}");
        }
    }
}
