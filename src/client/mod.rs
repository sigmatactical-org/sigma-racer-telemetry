//! Unix socket telemetry subscriber (sigma-racer-cluster).
//!
//! The reader runs on its own thread and **auto-reconnects**: if sigma-racer-vehicle
//! restarts (or the socket drops), the thread keeps retrying until the client is
//! dropped. Messages are delivered over a bounded channel so a slow UI cannot
//! cause unbounded memory growth — excess frames are dropped (the protocol sends
//! periodic full snapshots, so the UI re-syncs on the next one).

mod reader;
mod tcp;
mod tcp_reader;

use crate::protocol::{Message, SOCKET_PATH};
use reader::{CHANNEL_CAPACITY, configure, spawn_reader};
use std::io::Error;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;

pub use tcp::{DEFAULT_TCP_PORT as TCP_DEFAULT_PORT, TcpTelemetryClient, default_port};
pub use tcp_reader::RECONNECT_DELAY;

pub struct TelemetryClient {
    rx: Receiver<Message>,
    alive: Arc<AtomicBool>,
    _thread: JoinHandle<()>,
}

impl TelemetryClient {
    /// Connect to sigma-racer-vehicle; spawns a reader thread. Returns `None` if the
    /// service is not currently reachable (callers may retry later).
    pub fn connect() -> Option<Self> {
        Self::connect_path(default_socket())
    }

    pub fn connect_path(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref().to_path_buf();
        let stream = UnixStream::connect(&path).ok()?;
        configure(&stream);

        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let alive = Arc::new(AtomicBool::new(true));
        let alive_thread = Arc::clone(&alive);
        let thread = spawn_reader(path, Some(stream), tx, alive_thread);
        Some(Self {
            rx,
            alive,
            _thread: thread,
        })
    }

    pub fn try_recv(&self) -> Option<Message> {
        self.rx.try_recv().ok()
    }

    pub fn drain(&self) -> impl Iterator<Item = Message> + '_ {
        std::iter::from_fn(|| self.try_recv())
    }
}

impl Drop for TelemetryClient {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}

pub fn default_socket() -> String {
    std::env::var("SIGMA_RACER_WINGMAN_TELEMETRY_SOCKET")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| SOCKET_PATH.into())
}

pub fn connect_error(path: &Path, err: &Error) -> String {
    format!("telemetry: could not connect to {}: {err}", path.display())
}
