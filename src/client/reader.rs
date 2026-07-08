//! Background reader thread for the telemetry Unix socket.

use crate::protocol::Message;
use std::io::{BufRead, BufReader, ErrorKind};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::sync::mpsc::TrySendError;
use std::thread;
use std::time::Duration;

/// Bound on queued messages before the reader starts dropping frames.
pub const CHANNEL_CAPACITY: usize = 512;
/// Backoff between reconnect attempts when the service is down.
pub const RECONNECT_DELAY: Duration = Duration::from_millis(500);
/// Read timeout so the reader thread periodically observes shutdown requests.
pub const READ_TIMEOUT: Duration = Duration::from_millis(500);
/// Upper bound on a single NDJSON frame.
pub const MAX_LINE_BYTES: usize = 64 * 1024;

/// Outcome of reading a single connection.
enum Outcome {
    Stop,
    Reconnect,
}

pub fn configure(stream: &UnixStream) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
}

pub fn spawn_reader(
    path: PathBuf,
    initial: Option<UnixStream>,
    tx: SyncSender<Message>,
    alive: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || run(path, initial, tx, alive))
}

fn run(path: PathBuf, initial: Option<UnixStream>, tx: SyncSender<Message>, alive: Arc<AtomicBool>) {
    let mut stream = initial;
    while alive.load(Ordering::Relaxed) {
        let connection = match stream.take() {
            Some(s) => s,
            None => match UnixStream::connect(&path) {
                Ok(s) => {
                    configure(&s);
                    s
                }
                Err(_) => {
                    thread::sleep(RECONNECT_DELAY);
                    continue;
                }
            },
        };

        match read_stream(connection, &tx, &alive) {
            Outcome::Stop => return,
            Outcome::Reconnect => {
                if alive.load(Ordering::Relaxed) {
                    thread::sleep(RECONNECT_DELAY);
                }
            }
        }
    }
}

fn read_stream(stream: UnixStream, tx: &SyncSender<Message>, alive: &Arc<AtomicBool>) -> Outcome {
    let mut reader = BufReader::new(stream);
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
