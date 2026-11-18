// Snapshot of the game → Discord activity JSON.

use crate::api::{self, Snapshot};
use crate::config::{AirSpeed, Altitude, Config, Lang};
use crate::i18n::{names_column, tr};
use crate::names::{self, Names};
use serde_json::{json, Value};
use std::collections::HashSet;

pub const GAME_NAME: &str = "War Thunder";
const IMAGE_URL: &str = "https://static.encyclopedia.warthunder.com/images";

pub struct Ctx<'a> {
    pub cfg: &'a Config,
    pub lang: Lang,
    pub names: &'a Names,
    /// images uploaded to the Discord application
    pub assets: &'a HashSet<String>,
    /// unix seconds when the game was first seen
    pub start: u64,
}

/// Short human status for the tray, plus the activity (None = clear).
pub fn build(snap: &Snapshot, ctx: &Ctx) -> (String, Option<Value>) {
    let l = ctx.lang;
    let logo = ctx.cfg.logo.asset();
    let simple = |details: &str| {
        json!({
            "type": 0,
            "details": clamp(details),
            "timestamps": { "start": ctx.start },
            "assets": { "large_image": logo, "large_text": GAME_NAME },
        })
    };
    match snap {
        Snapshot::NotRunning => (tr(l, "game_not_running").into(), None),
        Snapshot::Hangar => (tr(l, "hangar").into(), Some(simple(tr(l, "hangar")))),
        Snapshot::Loading => (tr(l, "loading").into(), Some(simple(tr(l, "loading")))),
        Snapshot::Ground(ind) | Snapshot::Air(ind, _) => {
            let id = api::vehicle_id(&ind.vehicle);
            let name = names::lookup(ctx.names, &id, names_column(l));
            let details = format!("{}: {name}", tr(l, "plays_on"));
            let state = if !ctx.cfg.show_indicators {
                None
            } else if let Snapshot::Air(ind, st) = snap {
                air_state(ctx.cfg, l, ind, st.as_ref())
            } else {
                ground_state(l, ind)
            };
            let image = if ctx.cfg.custom_images && ctx.assets.contains(&id) { id.clone() } else { format!("{IMAGE_URL}/{id}.png") };
            let mut a = json!({
                "type": 0,
                "details": clamp(&details),
                "timestamps": { "start": ctx.start },
                "assets": {
                    "large_image": image,
                    "large_text": clamp(&name),
                    "small_image": logo,
                    "small_text": GAME_NAME,
                },
            });
            if let Some(s) = state {
                a["state"] = json!(clamp(&s));
            }
            (details, Some(a))
        }
    }
}

fn ground_state(l: Lang, ind: &api::Indicators) -> Option<String> {
    let mut parts = vec![];
    if let Some(s) = ind.speed {
        parts.push(format!("{}: {} {}", tr(l, "speed"), s.abs().round() as i64, tr(l, "kmh")));
    }
    if let (Some(cur), Some(total)) = (ind.crew_current, ind.crew_total) {
        parts.push(format!("{}: {}/{}", tr(l, "crew"), cur as i64, total as i64));
    }
    (!parts.is_empty()).then(|| parts.join(" | "))
}

fn air_state(cfg: &Config, l: Lang, ind: &api::Indicators, st: Option<&api::State>) -> Option<String> {
    let mut parts = vec![];
    let speed = match cfg.air_speed {
        AirSpeed::Ias => st.and_then(|s| s.ias).map(|v| (tr(l, "ias"), v)),
        AirSpeed::Tas => st.and_then(|s| s.tas).map(|v| (tr(l, "tas"), v)),
    };
    if let Some((label, v)) = speed {
        parts.push(format!("{label}: {} {}", v.round() as i64, tr(l, "kmh")));
    }
    let radio = ind.radio_altitude.filter(|_| cfg.altitude == Altitude::Radio).map(|v| (tr(l, "radio_alt"), v));
    if let Some((label, v)) = radio.or_else(|| st.and_then(|s| s.altitude).map(|v| (tr(l, "abs_alt"), v))) {
        parts.push(format!("{label}: {} {}", v.round() as i64, tr(l, "m")));
    }
    (!parts.is_empty()).then(|| parts.join(" | "))
}

/// Discord wants 2..=128 characters.
fn clamp(s: &str) -> String {
    let mut out: String = s.chars().take(128).collect();
    while out.chars().count() < 2 {
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Indicators, State};

    fn names() -> Names {
        crate::names::parse(r#"{"us_m1a1_aim_abrams":{"English":"M1A1 SA"},"su_27":{"English":"Su-27","Russian":"Су-27"}}"#).unwrap()
    }

    fn tank() -> Snapshot {
        Snapshot::Ground(Indicators {
            valid: true,
            army: "tank".into(),
            vehicle: "tankModels/us_m1a1_aim_abrams".into(),
            speed: Some(-37.6),
            crew_total: Some(4.0),
            crew_current: Some(3.0),
            ..Default::default()
        })
    }

    fn plane() -> Snapshot {
        let ind = Indicators { valid: true, army: "air".into(), vehicle: "su_27".into(), radio_altitude: Some(2870.4), ..Default::default() };
        let st = State { valid: true, tas: Some(690.0), ias: Some(612.0), altitude: Some(3050.0) };
        Snapshot::Air(ind, Some(st))
    }

    #[test]
    fn ground_presence() {
        let (cfg, n, a) = (Config::default(), names(), HashSet::new());
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 100 };
        let (status, act) = build(&tank(), &ctx);
        let act = act.unwrap();
        assert_eq!(status, "Plays on: M1A1 SA");
        assert_eq!(act["state"], "Speed: 38 km/h | Crew: 3/4");
        assert_eq!(act["assets"]["large_image"], format!("{IMAGE_URL}/us_m1a1_aim_abrams.png"));
        assert_eq!(act["assets"]["small_image"], "main_white");
        assert_eq!(act["timestamps"]["start"], 100);
    }

    #[test]
    fn air_presence_ru_with_custom_image() {
        let cfg = Config { air_speed: AirSpeed::Tas, ..Default::default() };
        let n = names();
        let a = HashSet::from(["su_27".to_string()]);
        let ctx = Ctx { cfg: &cfg, lang: Lang::Ru, names: &n, assets: &a, start: 0 };
        let act = build(&plane(), &ctx).1.unwrap();
        assert_eq!(act["details"], "Играет на: Су-27");
        assert_eq!(act["state"], "Скорость TAS: 690 км/ч | Радиовысота: 2870 м");
        assert_eq!(act["assets"]["large_image"], "su_27");
    }

    #[test]
    fn absolute_altitude_and_no_indicators() {
        let n = names();
        let a = HashSet::new();
        let cfg = Config { altitude: Altitude::Absolute, ..Default::default() };
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 0 };
        assert_eq!(build(&plane(), &ctx).1.unwrap()["state"], "IAS: 612 km/h | Altitude: 3050 m");
        let cfg = Config { show_indicators: false, ..Default::default() };
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 0 };
        assert!(build(&plane(), &ctx).1.unwrap().get("state").is_none());
    }

    #[test]
    fn hangar_and_offline() {
        let (cfg, n, a) = (Config::default(), names(), HashSet::new());
        let ctx = Ctx { cfg: &cfg, lang: Lang::Ru, names: &n, assets: &a, start: 0 };
        assert_eq!(build(&Snapshot::Hangar, &ctx).1.unwrap()["details"], "В ангаре");
        assert!(build(&Snapshot::NotRunning, &ctx).1.is_none());
    }
}
