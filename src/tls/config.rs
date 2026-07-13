//! TLS material paths and rustls config builders (TLS 1.3 + mTLS only).

use super::{CertPin, load_certs, load_private_key};
use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, ServerConfig, version::TLS13};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

/// Which role this process plays in telemetry mTLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsRole {
    /// Wingman relay (`sigma-telemetry-relay`).
    Server,
    /// Shop tool (`sigma-racer-mechanic`).
    Client,
}

/// PEM paths for the private Sigma telemetry PKI.
#[derive(Debug, Clone)]
pub struct TlsMaterial {
    pub ca_path: PathBuf,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    /// Optional SHA-256 (hex) of the Wingman server leaf certificate DER (client only).
    pub server_pin: Option<CertPin>,
}

impl TlsMaterial {
    pub fn load(role: TlsRole) -> Result<Self, String> {
        load_material(role)
    }

    /// Build material from explicit PEM paths (tests and provisioning tools).
    pub fn from_paths(
        ca_path: PathBuf,
        cert_path: PathBuf,
        key_path: PathBuf,
        server_pin: Option<CertPin>,
    ) -> Self {
        Self {
            ca_path,
            cert_path,
            key_path,
            server_pin,
        }
    }
}

pub fn load_material(role: TlsRole) -> Result<TlsMaterial, String> {
    let (ca_path, cert_path, key_path) = match role {
        TlsRole::Server => (
            env_path("TELEMETRY_TLS_CA", default_server_ca()),
            env_path("TELEMETRY_TLS_CERT", default_server_cert()),
            env_path("TELEMETRY_TLS_KEY", default_server_key()),
        ),
        TlsRole::Client => (
            env_path("TELEMETRY_TLS_CA", default_client_ca()),
            env_path("TELEMETRY_TLS_CERT", default_client_cert()),
            env_path("TELEMETRY_TLS_KEY", default_client_key()),
        ),
    };

    for path in [&ca_path, &cert_path, &key_path] {
        if !path.is_file() {
            return Err(format!(
                "TLS material missing: {} (telemetry requires TLS 1.3 mTLS)",
                path.display()
            ));
        }
    }

    let server_pin = std::env::var("TELEMETRY_TLS_SERVER_PIN")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| CertPin::parse_hex(&s))
        .transpose()?;

    Ok(TlsMaterial {
        ca_path,
        cert_path,
        key_path,
        server_pin,
    })
}

pub fn server_config(material: &TlsMaterial) -> Result<Arc<ServerConfig>, String> {
    let ca = load_certs(&material.ca_path)?;
    let mut roots = RootCertStore::empty();
    for cert in ca {
        roots
            .add(cert)
            .map_err(|e| format!("invalid CA {}: {e}", material.ca_path.display()))?;
    }

    // Require a client certificate signed by the telemetry CA (no anonymous clients).
    let client_verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(|e| format!("client verifier: {e}"))?;

    let certs = load_certs(&material.cert_path)?;
    let key = load_private_key(&material.key_path)?;

    let config = ServerConfig::builder_with_protocol_versions(&[&TLS13])
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)
        .map_err(|e| format!("server TLS config: {e}"))?;

    Ok(Arc::new(config))
}

pub fn client_config(material: &TlsMaterial) -> Result<Arc<ClientConfig>, String> {
    let ca = load_certs(&material.ca_path)?;
    let mut roots = RootCertStore::empty();
    for cert in ca {
        roots
            .add(cert)
            .map_err(|e| format!("invalid CA {}: {e}", material.ca_path.display()))?;
    }

    let client_certs = load_certs(&material.cert_path)?;
    let client_key = load_private_key(&material.key_path)?;

    let config = ClientConfig::builder_with_protocol_versions(&[&TLS13])
        .with_root_certificates(roots)
        .with_client_auth_cert(client_certs, client_key)
        .map_err(|e| format!("client TLS config: {e}"))?;

    Ok(Arc::new(config))
}

pub fn server_name_for_host(host: &str) -> Result<ServerName<'static>, String> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ServerName::IpAddress(ip.into()));
    }
    ServerName::try_from(host.to_string()).map_err(|_| format!("invalid TLS server name: {host}"))
}

fn env_path(var: &str, default: PathBuf) -> PathBuf {
    std::env::var(var)
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default)
}

fn default_server_ca() -> PathBuf {
    PathBuf::from("/etc/sigma-racer-wingman/telemetry-tls/ca.pem")
}

fn default_server_cert() -> PathBuf {
    PathBuf::from("/etc/sigma-racer-wingman/telemetry-tls/server.pem")
}

fn default_server_key() -> PathBuf {
    PathBuf::from("/etc/sigma-racer-wingman/telemetry-tls/server.key")
}

fn default_client_ca() -> PathBuf {
    home_config_dir().join("tls").join("ca.pem")
}

fn default_client_cert() -> PathBuf {
    home_config_dir().join("tls").join("client.pem")
}

fn default_client_key() -> PathBuf {
    home_config_dir().join("tls").join("client.key")
}

fn home_config_dir() -> PathBuf {
    std::env::var("SIGMA_RACER_MECHANIC_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME").map(|h| {
                PathBuf::from(h)
                    .join(".config")
                    .join("sigma-racer-mechanic")
            })
        })
        .unwrap_or_else(|_| PathBuf::from(".config/sigma-racer-mechanic"))
}
