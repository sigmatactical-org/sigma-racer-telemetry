//! VSS path mapping for [`VehicleState`](super::vehicle_state::VehicleState).

use serde_json::{Value, json};
use std::collections::HashMap;

use super::coerce::{json_bool, json_f32, json_i8, json_i16, json_i32, json_u8};
use super::vehicle_state::VehicleState;

impl VehicleState {
    pub fn to_vss_map(&self) -> HashMap<String, Value> {
        HashMap::from([
            ("Vehicle.Speed".into(), json!(self.speed.round() as i64)),
            (
                "Vehicle.Powertrain.CombustionEngine.Speed".into(),
                json!(self.rpm.round() as i64),
            ),
            (
                "Vehicle.Powertrain.Transmission.CurrentGear".into(),
                json!(self.gear),
            ),
            (
                "Vehicle.Powertrain.CombustionEngine.IsRedline".into(),
                json!(self.at_redline),
            ),
            (
                "Vehicle.Powertrain.CombustionEngine.ThrottlePosition".into(),
                json!(self.throttle_pct),
            ),
            (
                "Vehicle.Body.IsSideStandEngaged".into(),
                json!(self.side_stand),
            ),
            (
                "Vehicle.Powertrain.Transmission.PerformanceMode".into(),
                json!(self.riding_mode),
            ),
            ("Vehicle.FuelSystem.Level".into(), json!(self.fuel_pct)),
            (
                "Vehicle.OBD.CoolantTemperature".into(),
                json!(self.coolant_c),
            ),
            ("Vehicle.OBD.OilTemperature".into(), json!(self.oil_c)),
            ("Vehicle.TraveledDistance".into(), json!(self.odometer)),
            ("Vehicle.TripMeter1".into(), json!(self.trip1)),
            ("Vehicle.TripMeter2".into(), json!(self.trip2)),
            (
                "Vehicle.Acceleration.Lateral".into(),
                json!(self.lean_angle),
            ),
            (
                "Vehicle.Acceleration.Longitudinal".into(),
                json!(self.gforce),
            ),
            (
                "Vehicle.ElectricalSystem.Battery.Voltage".into(),
                json!(self.battery_v),
            ),
            (
                "Vehicle.Cabin.Infotainment.CanBusLoad".into(),
                json!(self.can_load),
            ),
            ("Vehicle.OBD.DTCCount".into(), json!(self.dtc)),
            ("Vehicle.ADAS.ABS.IsActive".into(), json!(self.abs_active)),
            ("Vehicle.ADAS.TCS.IsActive".into(), json!(self.tc_active)),
            (
                "Vehicle.CurrentLocation.Heading".into(),
                json!(self.heading.round() as i64),
            ),
            (
                "Vehicle.CurrentLocation.Altitude".into(),
                json!(self.elevation),
            ),
            (
                "Vehicle.Service.SignalsLive".into(),
                json!(self.signals_live),
            ),
        ])
    }

    pub fn apply_vss(&mut self, path: &str, value: &Value) {
        match path {
            "Vehicle.Speed" => self.speed = json_f32(value),
            "Vehicle.Powertrain.CombustionEngine.Speed" => self.rpm = json_f32(value),
            "Vehicle.Powertrain.Transmission.CurrentGear" => self.gear = json_i8(value),
            "Vehicle.Powertrain.CombustionEngine.IsRedline" => {}
            "Vehicle.Powertrain.CombustionEngine.ThrottlePosition" => {
                self.throttle_pct = json_f32(value)
            }
            "Vehicle.Body.IsSideStandEngaged" => self.side_stand = json_bool(value),
            "Vehicle.Powertrain.Transmission.PerformanceMode" => {
                if let Some(s) = value.as_str() {
                    self.riding_mode = s.into();
                }
            }
            "Vehicle.FuelSystem.Level" => self.fuel_pct = json_f32(value),
            "Vehicle.OBD.CoolantTemperature" => self.coolant_c = json_i16(value),
            "Vehicle.OBD.OilTemperature" => self.oil_c = json_i16(value),
            "Vehicle.TraveledDistance" => self.odometer = json_f32(value),
            "Vehicle.TripMeter1" => self.trip1 = json_f32(value),
            "Vehicle.TripMeter2" => self.trip2 = json_f32(value),
            "Vehicle.Acceleration.Lateral" => self.lean_angle = json_f32(value),
            "Vehicle.Acceleration.Longitudinal" => self.gforce = json_f32(value),
            "Vehicle.ElectricalSystem.Battery.Voltage" => self.battery_v = json_f32(value),
            "Vehicle.Cabin.Infotainment.CanBusLoad" => self.can_load = json_u8(value),
            "Vehicle.OBD.DTCCount" => self.dtc = json_u8(value),
            "Vehicle.ADAS.ABS.IsActive" => self.abs_active = json_bool(value),
            "Vehicle.ADAS.TCS.IsActive" => self.tc_active = json_bool(value),
            "Vehicle.CurrentLocation.Heading" => self.heading = json_f32(value),
            "Vehicle.CurrentLocation.Altitude" => self.elevation = json_i32(value),
            "Vehicle.Service.SignalsLive" => self.signals_live = json_bool(value),
            _ => {}
        }
    }

    pub fn apply_vss_map(&mut self, data: &HashMap<String, Value>) {
        for (path, value) in data {
            self.apply_vss(path, value);
        }
        self.refresh_derived();
    }
}
