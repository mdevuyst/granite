use pingora::tls::pkey::{PKey, Private};
use pingora::tls::x509::X509;
use serde::{Deserialize, Serialize};

/// An interface to add and delete certificates and their bindings.
pub trait CertHolder: Send + Sync {
    fn add_cert(&self, host: &str, cert: X509, key: PKey<Private>);
    fn delete_cert(&self, host: &str);
}

/// A binding associates a hostname with a certificate and key.
/// During a TLS handshake, the client sends the hostname it's trying to connect to in the SNI
/// and the proxy selects the appropriate certificate and key by searching for the matching binding.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CertBinding {
    /// The hostname/SNI associated with the certificate and key.
    pub host: String,

    /// An X509 server certificate in in a string in PEM format.
    pub cert: String,

    /// The corresponding private key in a string in PEM format.
    pub key: String,
}
