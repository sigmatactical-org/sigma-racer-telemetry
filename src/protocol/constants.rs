//! NDJSON telemetry protocol constants.

pub const PROTOCOL_VERSION: &str = "0.1";
pub const SOCKET_PATH: &str = "/run/sigma-racer-wingman/vehicle.sock";

/// Full snapshot rate when nothing changed (10 Hz per schema).
pub const SNAPSHOT_INTERVAL_MS: u64 = 100;
