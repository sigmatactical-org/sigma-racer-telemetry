//! JSON value coercion for VSS apply paths.

use serde_json::Value;

pub fn json_f32(v: &Value) -> f32 {
    v.as_f64().unwrap_or(0.0) as f32
}

pub fn json_i8(v: &Value) -> i8 {
    v.as_i64().unwrap_or(0).clamp(-128, 127) as i8
}

pub fn json_i16(v: &Value) -> i16 {
    v.as_i64().unwrap_or(0).clamp(-32768, 32767) as i16
}

pub fn json_i32(v: &Value) -> i32 {
    v.as_i64().unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

pub fn json_u8(v: &Value) -> u8 {
    v.as_u64().unwrap_or(0).min(255) as u8
}

pub fn json_bool(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}
