//! Unix domain socket setup for sigma-racer-vehicle telemetry publisher.

mod permissions;

pub use permissions::{bind_listener, prepare_socket_path, RUNTIME_DIR_MODE, SOCKET_MODE};
