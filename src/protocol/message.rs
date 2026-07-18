//! NDJSON telemetry envelope (schemas/telemetry/vehicle-messages.yaml v0.1).

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::state::VehicleState;

use super::constants::PROTOCOL_VERSION;
use super::parse_error::ParseError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub version: String,
    pub msg: String,
    pub ts: String,
    pub seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vss: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uptime_ms: Option<u64>,
}

impl Message {
    /// Full-state snapshot message.
    pub fn snapshot(seq: u64, state: &VehicleState) -> Self {
        Self {
            version: PROTOCOL_VERSION.into(),
            msg: "Snapshot".into(),
            ts: now_iso(),
            seq,
            data: Some(state.to_vss_map()),
            event: None,
            vss: None,
            value: None,
            uptime_ms: None,
        }
    }

    /// Delta update carrying only changed VSS entries.
    pub fn signal_update(seq: u64, data: HashMap<String, Value>) -> Self {
        Self {
            version: PROTOCOL_VERSION.into(),
            msg: "SignalUpdate".into(),
            ts: now_iso(),
            seq,
            data: Some(data),
            event: None,
            vss: None,
            value: None,
            uptime_ms: None,
        }
    }

    /// Edge-triggered event (anomaly raise/clear, mode change). Unlike the
    /// self-stamping constructors, `ts` is explicit: an event carries the
    /// timestamp of the sample that triggered it, so replayed sessions
    /// reproduce identical events.
    pub fn event(
        seq: u64,
        ts: String,
        event: &str,
        vss: &str,
        value: Value,
        data: HashMap<String, Value>,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION.into(),
            msg: "Event".into(),
            ts,
            seq,
            data: Some(data),
            event: Some(event.into()),
            vss: Some(vss.into()),
            value: Some(value),
            uptime_ms: None,
        }
    }

    /// Liveness message sent when nothing else is flowing.
    pub fn heartbeat(seq: u64, uptime_ms: u64) -> Self {
        Self {
            version: PROTOCOL_VERSION.into(),
            msg: "Heartbeat".into(),
            ts: now_iso(),
            seq,
            data: None,
            event: None,
            vss: None,
            value: None,
            uptime_ms: Some(uptime_ms),
        }
    }

    /// Serialize as one NDJSON line (no trailing newline).
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("telemetry message serializes")
    }

    /// Parse one NDJSON line without semantic validation.
    pub fn parse_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim())
    }

    /// Parse one NDJSON line and validate version/type/length bounds.
    pub fn parse_validated(line: &str) -> Result<Self, ParseError> {
        let msg = Self::parse_line(line).map_err(ParseError::Json)?;
        if msg.version != PROTOCOL_VERSION {
            return Err(ParseError::UnsupportedVersion(msg.version));
        }
        match msg.msg.as_str() {
            "Snapshot" | "SignalUpdate" | "Heartbeat" | "Event" => Ok(msg),
            other => Err(ParseError::UnknownKind(other.into())),
        }
    }

    /// The VSS payload for snapshot/update messages, `None` otherwise.
    pub fn vss_data(&self) -> Option<&HashMap<String, Value>> {
        self.data.as_ref()
    }
}

/// Current UTC time in RFC 3339 with milliseconds (message timestamps).
pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::VehicleState;

    #[test]
    fn rejects_wrong_protocol_version() {
        let line = r#"{"version":"9.9","msg":"Snapshot","ts":"t","seq":1,"data":{}}"#;
        match Message::parse_validated(line) {
            Err(ParseError::UnsupportedVersion(v)) => assert_eq!(v, "9.9"),
            other => panic!("expected version error, got {other:?}"),
        }
    }

    #[test]
    fn accepts_snapshot() {
        let line = Message::snapshot(1, &VehicleState::idle()).to_line();
        assert!(Message::parse_validated(&line).is_ok());
    }

    #[test]
    fn event_round_trips_through_validation() {
        let mut data = HashMap::new();
        data.insert("state".to_string(), Value::from("raised"));
        data.insert("severity".to_string(), Value::from("CRITICAL"));
        data.insert("message".to_string(), Value::from("Coolant 118 °C"));
        let msg = Message::event(
            7,
            "2026-07-18T10:00:00.000Z".into(),
            "coolant_overheat",
            "Vehicle.OBD.CoolantTemperature",
            Value::from(118),
            data,
        );
        let parsed = Message::parse_validated(&msg.to_line()).expect("event parses");
        assert_eq!(parsed.msg, "Event");
        assert_eq!(parsed.event.as_deref(), Some("coolant_overheat"));
        assert_eq!(parsed.ts, "2026-07-18T10:00:00.000Z");
        assert_eq!(parsed.value, Some(Value::from(118)));
        assert_eq!(
            parsed.data.as_ref().and_then(|d| d.get("state")),
            Some(&Value::from("raised"))
        );
    }

    #[test]
    fn still_rejects_unknown_kind() {
        let line = r#"{"version":"0.1","msg":"Bogus","ts":"t","seq":1}"#;
        match Message::parse_validated(line) {
            Err(ParseError::UnknownKind(k)) => assert_eq!(k, "Bogus"),
            other => panic!("expected unknown kind, got {other:?}"),
        }
    }
}
