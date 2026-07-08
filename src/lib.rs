//! Sigma Racer vehicle telemetry — VSS state, CAN decode (M7 draft), JSON/NDJSON IPC.

pub mod can;
pub mod client;
pub mod m7_dbc;
pub mod protocol;
pub mod socket;
pub mod state;

pub use client::TelemetryClient;
pub use protocol::{Message, ParseError, SOCKET_PATH, SNAPSHOT_INTERVAL_MS};
pub use socket::{bind_listener, prepare_socket_path};
pub use state::VehicleState;
