# sigma-racer-telemetry

VSS vehicle state, M7 CAN bridge, and NDJSON IPC for Sigma Racer cockpit services.

## Crate

- **`sigma-racer-telemetry`** — `VehicleState`, Unix-socket telemetry protocol, publisher helpers, and `TelemetryClient` for subscribers.

## Consumers

- **sigma-racer-vehicle** — CAN/sim → VSS → `/run/sigma-racer-wingman/vehicle.sock`
- **sigma-racer-cluster** — Slint dashboard subscriber

## Test

```bash
cargo test
```
