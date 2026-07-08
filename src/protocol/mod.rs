//! NDJSON telemetry envelopes (schemas/telemetry/vehicle-messages.yaml v0.1).

mod constants;
mod diff;
mod message;
mod parse_error;

pub use constants::{PROTOCOL_VERSION, SNAPSHOT_INTERVAL_MS, SOCKET_PATH};
pub use diff::diff_vss;
pub use message::{now_iso, Message};
pub use parse_error::ParseError;
