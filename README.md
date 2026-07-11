# sigma-racer-telemetry

[![CI](https://github.com/sigmatactical-org/sigma-racer-telemetry/actions/workflows/ci.yml/badge.svg)](https://github.com/sigmatactical-org/sigma-racer-telemetry/actions/workflows/ci.yml)

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

## Brand & artwork

© Sigma Tactical Group. **All rights reserved.**

The Sigma Tactical Group name, logos, marks, artwork, and visual identity are **proprietary**. They are not covered by this repository's source-code license. See [BRANDING.md](BRANDING.md).
