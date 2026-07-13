//! TLS 1.3 + mTLS for Wingman telemetry relay and Mechanic clients.

mod config;
mod pin;

pub use config::{
    TlsMaterial, TlsRole, client_config, load_material, server_config, server_name_for_host,
};
pub use pin::{CertPin, verify_server_pin};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConnection, ServerConnection, StreamOwned};
use std::fs::File;
use std::io::{self, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;

pub type TlsServerStream = StreamOwned<ServerConnection, TcpStream>;
pub type TlsClientStream = StreamOwned<ClientConnection, TcpStream>;

/// Perform a blocking server-side TLS handshake (mTLS required).
pub fn accept_tls(
    config: &Arc<rustls::ServerConfig>,
    mut tcp: TcpStream,
) -> Result<TlsServerStream, String> {
    let mut conn = ServerConnection::new(Arc::clone(config))
        .map_err(|e| format!("TLS server connection: {e}"))?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)
            .map_err(|e| format!("TLS server handshake: {e}"))?;
    }
    Ok(StreamOwned::new(conn, tcp))
}

/// Connect and perform a blocking client-side TLS handshake (mTLS + optional pin).
pub fn connect_tls(
    config: &Arc<rustls::ClientConfig>,
    server_name: rustls::pki_types::ServerName<'static>,
    host: &str,
    port: u16,
    pin: Option<&CertPin>,
) -> Result<TlsClientStream, String> {
    let addr = format!("{host}:{port}");
    let mut tcp = TcpStream::connect(&addr).map_err(|e| format!("TCP connect {addr}: {e}"))?;
    let mut conn = ClientConnection::new(Arc::clone(config), server_name)
        .map_err(|e| format!("TLS client connection: {e}"))?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)
            .map_err(|e| format!("TLS client handshake: {e}"))?;
    }

    if let Some(pin) = pin {
        let peer = conn
            .peer_certificates()
            .ok_or_else(|| "TLS peer did not present a certificate".to_string())?;
        verify_server_pin(peer, pin)?;
    }

    Ok(StreamOwned::new(conn, tcp))
}

pub fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
    let file = File::open(path).map_err(|e| format!("open cert {}: {e}", path.display()))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("parse cert {}: {e}", path.display()))
}

pub fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, String> {
    let file = File::open(path).map_err(|e| format!("open key {}: {e}", path.display()))?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("parse key {}: {e}", path.display()))?
        .ok_or_else(|| format!("no private key in {}", path.display()))
}

pub fn set_read_timeout(stream: &TcpStream, timeout: std::time::Duration) {
    let _ = stream.set_read_timeout(Some(timeout));
}

pub fn set_nonblocking(stream: &TcpStream, nonblocking: bool) -> io::Result<()> {
    stream.set_nonblocking(nonblocking)
}

/// Read timeout helper for TLS streams (uses underlying TCP socket).
pub fn tls_read_timeout(stream: &mut TlsClientStream, timeout: std::time::Duration) {
    let _ = stream.get_ref().set_read_timeout(Some(timeout));
}

pub fn tls_set_nonblocking(stream: &mut TlsServerStream, nonblocking: bool) -> io::Result<()> {
    stream.get_mut().set_nonblocking(nonblocking)
}
