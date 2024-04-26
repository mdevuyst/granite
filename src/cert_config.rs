use pingora::tls::pkey::{PKey, Private};
use pingora::tls::x509::X509;
use serde::{Deserialize, Serialize};

pub trait CertHolder: Send + Sync {
    fn add_cert(&self, host: &str, cert: X509, key: PKey<Private>);
    fn delete_cert(&self, host: &str);
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CertBinding {
    pub host: String,
    pub cert: String,
    pub key: String,
}
