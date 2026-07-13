//! mTLS telemetry subscriber (Mechanic / shop tools over WiFi).

use super::tcp_reader::{CHANNEL_CAPACITY, spawn_tls_reader};
use crate::protocol::Message;
use crate::tls::{TlsRole, client_config, load_material, server_name_for_host};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;

/// Default TCP port for Wingman telemetry relay (TLS wrapper).
pub const DEFAULT_TCP_PORT: u16 = 7357;

pub struct TcpTelemetryClient {
    rx: Receiver<Message>,
    alive: Arc<AtomicBool>,
    _thread: JoinHandle<()>,
}

impl TcpTelemetryClient {
    /// Connect with TLS 1.3 + mTLS using PEM material from env / default paths.
    pub fn connect(host: &str, port: u16) -> Result<Self, String> {
        install_crypto();
        let material = load_material(TlsRole::Client)?;
        let config = client_config(&material)?;
        let server_name = server_name_for_host(host)?;
        let pin = material.server_pin.clone();

        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let alive = Arc::new(AtomicBool::new(true));
        let alive_thread = Arc::clone(&alive);
        let host = host.to_string();
        let thread = spawn_tls_reader(host, port, config, pin, server_name, tx, alive_thread);
        Ok(Self {
            rx,
            alive,
            _thread: thread,
        })
    }

    pub fn connect_default(host: &str) -> Result<Self, String> {
        Self::connect(host, default_port())
    }

    pub fn try_recv(&self) -> Option<Message> {
        self.rx.try_recv().ok()
    }

    pub fn drain(&self) -> impl Iterator<Item = Message> + '_ {
        std::iter::from_fn(|| self.try_recv())
    }
}

impl Drop for TcpTelemetryClient {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}

pub fn default_port() -> u16 {
    std::env::var("TELEMETRY_TCP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TCP_PORT)
}

fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
