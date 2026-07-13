//! VSS diff helpers for incremental updates.

use crate::state::VehicleState;
use serde_json::Value;
use std::collections::HashMap;

pub fn diff_vss(prev: &VehicleState, next: &VehicleState) -> HashMap<String, Value> {
    let a = prev.to_vss_map();
    let b = next.to_vss_map();
    b.into_iter().filter(|(k, v)| a.get(k) != Some(v)).collect()
}
