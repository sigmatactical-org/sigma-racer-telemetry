//! Background reader thread for mTLS telemetry.

use crate::protocol::Message;
use crate::tls::{CertPin, connect_tls, tls_read_timeout};
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::thread;
use std::time::Duration;

pub const CHANNEL_CAPACITY: usize = 512;
pub const RECONNECT_DELAY: Duration = Duration::from_millis(500);
pub const READ_TIMEOUT: Duration = Duration::from_millis(500);
pub const MAX_LINE_BYTES: usize = 64 * 1024;

enum Outcome {
    Stop,
    Reconnect,
}

pub fn spawn_tls_reader(
    host: String,
    port: u16,
    config: Arc<ClientConfig>,
    pin: Option<CertPin>,
    server_name: ServerName<'static>,
    tx: SyncSender<Message>,
    alive: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || run(host, port, config, pin, server_name, tx, alive))
}

fn run(
    host: String,
    port: u16,
    config: Arc<ClientConfig>,
    pin: Option<CertPin>,
    server_name: ServerName<'static>,
    tx: SyncSender<Message>,
    alive: Arc<AtomicBool>,
) {
    while alive.load(Ordering::Relaxed) {
        match connect_tls(&config, server_name.clone(), &host, port, pin.as_ref()) {
            Ok(mut stream) => {
                tls_read_timeout(&mut stream, READ_TIMEOUT);
                match read_stream(stream, &tx, &alive) {
                    Outcome::Stop => return,
                    Outcome::Reconnect => {}
                }
            }
            Err(err) => {
                eprintln!("telemetry: mTLS connect failed: {err}");
            }
        }
        if alive.load(Ordering::Relaxed) {
            thread::sleep(RECONNECT_DELAY);
        }
    }
}

fn read_stream<R: Read>(
    mut stream: R,
    tx: &SyncSender<Message>,
    alive: &Arc<AtomicBool>,
) -> Outcome {
    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();
    loop {
        if !alive.load(Ordering::Relaxed) {
            return Outcome::Stop;
        }
        match reader.read_line(&mut line) {
            Ok(0) => return Outcome::Reconnect,
            Ok(_) => {
                if !line.ends_with('\n') {
                    if line.len() > MAX_LINE_BYTES {
                        line.clear();
                    }
                    continue;
                }
                match Message::parse_validated(&line) {
                    Ok(msg) => match tx.try_send(msg) {
                        Ok(()) => {}
                        Err(TrySendError::Full(_)) => {}
                        Err(TrySendError::Disconnected(_)) => return Outcome::Stop,
                    },
                    Err(err) => eprintln!("telemetry: ignore malformed frame: {err}"),
                }
                line.clear();
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => return Outcome::Reconnect,
        }
    }
}
