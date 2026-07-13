//! mTLS relay: subscribe to the local Unix telemetry socket and fan out NDJSON.

mod broadcaster;

use crate::protocol::SOCKET_PATH;
use crate::tls::{TlsServerStream, accept_tls};
use broadcaster::TlsLineBroadcaster;
use rustls::ServerConfig;
use std::io;
use std::io::{BufRead, BufReader, ErrorKind};
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub use broadcaster::{TcpLineBroadcaster, TlsLineBroadcaster as Broadcaster};

/// Default TCP port for shop-tool telemetry (Mechanic). Traffic is TLS 1.3 + mTLS only.
pub const DEFAULT_TCP_PORT: u16 = 7357;

const RECONNECT_DELAY: Duration = Duration::from_millis(500);
const READ_TIMEOUT: Duration = Duration::from_millis(500);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);

/// Run until interrupted. Forwards NDJSON from `socket_path` to mTLS clients on `listen_addr`.
pub fn run(listen_addr: &str, socket_path: &Path, tls: Arc<ServerConfig>) -> io::Result<()> {
    let listener = TcpListener::bind(listen_addr)?;
    listener.set_nonblocking(true)?;
    eprintln!(
        "sigma-telemetry-relay: mTLS listening on {listen_addr}, source {}",
        socket_path.display()
    );

    let (line_tx, line_rx) = mpsc::channel::<String>();
    let (client_tx, client_rx) = mpsc::channel::<TlsServerStream>();
    spawn_unix_subscriber(socket_path.to_path_buf(), line_tx);

    let mut clients = TlsLineBroadcaster::new();
    loop {
        accept_tls_clients(&listener, Arc::clone(&tls), client_tx.clone());
        while let Ok(stream) = client_rx.try_recv() {
            clients.add(stream);
        }
        while let Ok(line) = line_rx.try_recv() {
            clients.send_line(&line);
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn accept_tls_clients(
    listener: &TcpListener,
    tls: Arc<ServerConfig>,
    client_tx: mpsc::Sender<TlsServerStream>,
) {
    loop {
        match listener.accept() {
            Ok((tcp, addr)) => {
                let cfg = Arc::clone(&tls);
                let tx = client_tx.clone();
                thread::spawn(move || {
                    let _ = tcp.set_read_timeout(Some(TLS_HANDSHAKE_TIMEOUT));
                    match accept_tls(&cfg, tcp) {
                        Ok(stream) => {
                            eprintln!("sigma-telemetry-relay: mTLS client {addr}");
                            let _ = tx.send(stream);
                        }
                        Err(e) => {
                            eprintln!("sigma-telemetry-relay: rejected {addr}: {e}");
                        }
                    }
                });
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(e) => {
                eprintln!("sigma-telemetry-relay: accept: {e}");
                break;
            }
        }
    }
}

fn spawn_unix_subscriber(socket_path: PathBuf, line_tx: mpsc::Sender<String>) {
    thread::spawn(move || {
        loop {
            if let Ok(stream) = UnixStream::connect(&socket_path) {
                let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
                eprintln!(
                    "sigma-telemetry-relay: connected to {}",
                    socket_path.display()
                );
                if read_unix_lines(stream, &line_tx).is_err() {
                    eprintln!("sigma-telemetry-relay: unix socket dropped, reconnecting");
                }
            }
            thread::sleep(RECONNECT_DELAY);
        }
    });
}

fn read_unix_lines(stream: UnixStream, line_tx: &mpsc::Sender<String>) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(io::Error::from(ErrorKind::UnexpectedEof)),
            Ok(_) => {
                if line.ends_with('\n') && !line.trim().is_empty() {
                    let _ = line_tx.send(line.clone());
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

pub fn default_socket_path() -> PathBuf {
    std::env::var("SIGMA_RACER_WINGMAN_TELEMETRY_SOCKET")
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(SOCKET_PATH))
}

pub fn default_listen_addr() -> String {
    let port = std::env::var("TELEMETRY_RELAY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TCP_PORT);
    format!("0.0.0.0:{port}")
}
