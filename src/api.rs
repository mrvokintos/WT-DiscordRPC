// War Thunder local API (localhost:8111), the same endpoints the game's web map uses.

use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Indicators {
    #[serde(default)]
    pub valid: bool,
    /// "tank", "air" or "dummy_plane" while loading
    #[serde(default)]
    pub army: String,
    /// vehicle id; "tankModels/<id>" for ground vehicles
    #[serde(default, rename = "type")]
    pub vehicle: String,
    pub speed: Option<f64>,
    pub crew_total: Option<f64>,
    pub crew_current: Option<f64>,
    pub radio_altitude: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct State {
    #[serde(default)]
    pub valid: bool,
    #[serde(rename = "TAS, km/h")]
    pub tas: Option<f64>,
    #[serde(rename = "IAS, km/h")]
    pub ias: Option<f64>,
    #[serde(rename = "H, m")]
    pub altitude: Option<f64>,
}

/// What the game is doing right now.
#[derive(Debug, Clone)]
pub enum Snapshot {
    NotRunning,
    Hangar,
    Loading,
    /// battle map is up but there is no vehicle: respawn screen, spectating, after death
    InBattle,
    Ground(Indicators),
    Air(Indicators, Option<State>),
    /// ships and anything else: name and picture only
    Other(Indicators),
}

pub struct Api {
    agent: ureq::Agent,
    base: String,
}

impl Api {
    pub fn new(base: String) -> Self {
        Self { agent: ureq::AgentBuilder::new().timeout(Duration::from_millis(800)).build(), base }
    }

    fn get(&self, path: &str) -> Result<Value, ureq::Error> {
        Ok(self.agent.get(&format!("{}{path}", self.base)).call()?.into_json()?)
    }

    pub fn snapshot(&self, want_state: bool) -> Snapshot {
        let map = match self.get("/map_info.json") {
            Ok(v) => v,
            Err(ureq::Error::Transport(_)) => return Snapshot::NotRunning,
            Err(e) => {
                log::debug!("map_info: {e}");
                return Snapshot::Hangar;
            }
        };
        if !map.get("valid").and_then(Value::as_bool).unwrap_or(false) {
            return Snapshot::Hangar;
        }
        let ind: Indicators = match self.get("/indicators").map(serde_json::from_value) {
            Ok(Ok(i)) => i,
            other => {
                log::debug!("indicators: {other:?}");
                return Snapshot::InBattle;
            }
        };
        let air = ind.army == "air";
        let state = (air && want_state).then(|| self.get("/state").ok().and_then(|v| serde_json::from_value(v).ok())).flatten();
        classify(ind, state)
    }
}

pub fn classify(ind: Indicators, state: Option<State>) -> Snapshot {
    if ind.army == "dummy_plane" || ind.vehicle == "dummy_plane" {
        return Snapshot::Loading;
    }
    if !ind.valid || ind.vehicle.is_empty() {
        return Snapshot::InBattle;
    }
    match ind.army.as_str() {
        "tank" => Snapshot::Ground(ind),
        "air" => Snapshot::Air(ind, state.filter(|s| s.valid)),
        _ => Snapshot::Other(ind),
    }
}

/// Vehicle id as used by the encyclopedia and names.json.
pub fn vehicle_id(raw: &str) -> String {
    raw.rsplit('/').next().unwrap_or(raw).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn ground_indicators() {
        let ind: Indicators = serde_json::from_value(fixture("indicators_tank")).unwrap();
        assert_eq!(vehicle_id(&ind.vehicle), "us_m1a1_aim_abrams");
        assert!(matches!(classify(ind, None), Snapshot::Ground(i) if i.crew_total == Some(4.0)));
    }

    #[test]
    fn air_indicators_and_state() {
        let ind: Indicators = serde_json::from_value(fixture("indicators_air")).unwrap();
        let st: State = serde_json::from_value(fixture("state_air")).unwrap();
        assert_eq!(st.ias, Some(612.0));
        assert!(matches!(classify(ind, Some(st)), Snapshot::Air(_, Some(_))));
    }

    #[test]
    fn dummy_plane_is_loading() {
        let ind: Indicators = serde_json::from_value(fixture("indicators_dummy")).unwrap();
        assert!(matches!(classify(ind, None), Snapshot::Loading));
        let invalid: Indicators = serde_json::from_value(serde_json::json!({ "valid": false })).unwrap();
        assert!(matches!(classify(invalid, None), Snapshot::InBattle));
        let ship: Indicators = serde_json::from_value(serde_json::json!({ "valid": true, "army": "ship", "type": "us_destroyer_fletcher" })).unwrap();
        assert!(matches!(classify(ship, None), Snapshot::Other(_)));
    }
}
