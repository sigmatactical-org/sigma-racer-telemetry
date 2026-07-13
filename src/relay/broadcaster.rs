//! Fan-out NDJSON lines to connected mTLS clients.

use crate::tls::{TlsServerStream, tls_set_nonblocking};
use std::collections::VecDeque;
use std::io::{ErrorKind, Write};

const MAX_CLIENT_BACKLOG: usize = 256 * 1024;

struct Client {
    stream: TlsServerStream,
    pending: VecDeque<u8>,
}

impl Client {
    fn enqueue_and_flush(&mut self, bytes: &[u8]) -> bool {
        self.pending.extend(bytes.iter().copied());
        self.flush()
    }

    fn flush(&mut self) -> bool {
        while !self.pending.is_empty() {
            let (head, _) = self.pending.as_slices();
            match self.stream.write(head) {
                Ok(0) => return false,
                Ok(n) => {
                    self.pending.drain(..n);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    return self.pending.len() <= MAX_CLIENT_BACKLOG;
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(_) => return false,
            }
        }
        true
    }
}

pub struct TlsLineBroadcaster {
    clients: Vec<Client>,
}

impl TlsLineBroadcaster {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    pub fn add(&mut self, mut stream: TlsServerStream) {
        if tls_set_nonblocking(&mut stream, true).is_err() {
            return;
        }
        self.clients.push(Client {
            stream,
            pending: VecDeque::new(),
        });
    }

    pub fn send_line(&mut self, line: &str) {
        let mut bytes = line.to_string();
        if !bytes.ends_with('\n') {
            bytes.push('\n');
        }
        self.clients
            .retain_mut(|client| client.enqueue_and_flush(bytes.as_bytes()));
    }
}

impl Default for TlsLineBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

// Backward-compatible alias for relay module.
pub type TcpLineBroadcaster = TlsLineBroadcaster;
