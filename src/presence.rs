// Snapshot of the game → Discord activity JSON.

use crate::api::{self, Snapshot};
use crate::config::{AirSpeed, Altitude, Config, Lang};
use crate::battle::Battle;
use crate::i18n::{self, names_column, tr};
use crate::names::{self, Names};
use serde_json::{json, Value};
use std::collections::HashSet;

pub const GAME_NAME: &str = "War Thunder";
const IMAGE_URL: &str = "https://static.encyclopedia.warthunder.com/images";
const WIKI_URL: &str = "https://wiki.warthunder.com/unit";
/// images in this repository, used when the Discord application has no asset of that name
const REPO_ASSETS: &str = "https://raw.githubusercontent.com/mrvokintos/WT-DiscordRPC/main/assets";

pub struct Ctx<'a> {
    pub cfg: &'a Config,
    pub lang: Lang,
    pub names: &'a Names,
    /// images uploaded to the Discord application
    pub assets: &'a HashSet<String>,
    /// unix seconds when the game was first seen
    pub start: u64,
    pub battle: &'a Battle,
}

impl Ctx<'_> {
    /// Asset key if uploaded to the application, else a link to the repository copy.
    fn image(&self, key: &str, dir: &str) -> String {
        if self.assets.contains(key) {
            key.to_string()
        } else {
            // Discord's image proxy caches per URL, a failed fetch too: the version makes a new URL
            format!("{REPO_ASSETS}/{dir}/{key}.png?v={}", env!("CARGO_PKG_VERSION"))
        }
    }

    fn logo(&self) -> String {
        self.image(self.cfg.logo.asset(), "logo")
    }

    fn timer_start(&self) -> u64 {
        if self.cfg.battle_timer && self.battle.active { self.battle.start } else { self.start }
    }

    fn kills(&self) -> Option<String> {
        (self.cfg.show_kills && !self.cfg.nickname.is_empty() && self.battle.active)
            .then(|| format!("{}: {}", tr(self.lang, "kills"), self.battle.kills))
    }
}

/// Short human status for the tray, plus the activity (None = clear).
pub fn build(snap: &Snapshot, ctx: &Ctx) -> (String, Option<Value>) {
    let l = ctx.lang;
    let logo = ctx.logo();
    let finish = |mut a: Value, state: Option<String>, id: Option<&str>| {
        if let Some(s) = state.filter(|s| !s.is_empty()) {
            a["state"] = json!(clamp(&s));
        }
        let mut b = vec![];
        if let Some(id) = id.filter(|_| ctx.cfg.wiki_button) {
            b.push(json!({ "label": tr(l, "wiki_button"), "url": format!("{WIKI_URL}/{id}") }));
        }
        if ctx.cfg.repo_button {
            b.push(json!({ "label": "WT Discord RPC", "url": format!("https://github.com/{}", crate::update::REPO) }));
        }
        if !b.is_empty() {
            a["buttons"] = json!(b);
        }
        a
    };
    let simple = |details: &str, state: Option<String>| {
        let a = json!({
            "type": 0,
            "details": clamp(details),
            "timestamps": { "start": ctx.timer_start() },
            "assets": { "large_image": logo, "large_text": GAME_NAME },
        });
        finish(a, state, None)
    };
    match snap {
        Snapshot::NotRunning => (tr(l, "game_not_running").into(), None),
        Snapshot::Hangar => (tr(l, "hangar").into(), Some(simple(tr(l, "hangar"), None))),
        Snapshot::Loading => (tr(l, "loading").into(), Some(simple(tr(l, "loading"), None))),
        Snapshot::InBattle => (tr(l, "in_battle").into(), Some(simple(tr(l, "in_battle"), ctx.kills()))),
        Snapshot::Ground(ind) | Snapshot::Air(ind, _) | Snapshot::Other(ind) => {
            let id = api::vehicle_id(&ind.vehicle);
            let name = names::lookup(ctx.names, &id, names_column(l));
            let details = format!("{}: {name}", tr(l, "plays_on"));
            let indicators = if !ctx.cfg.show_indicators {
                None
            } else if let Snapshot::Air(ind, st) = snap {
                air_state(ctx.cfg, l, ind, st.as_ref())
            } else if let Snapshot::Ground(ind) = snap {
                ground_state(l, ind)
            } else {
                None
            };
            let state: Vec<String> = indicators.into_iter().chain(ctx.kills()).collect();
            let image = if ctx.cfg.custom_images && ctx.assets.contains(&id) { id.clone() } else { format!("{IMAGE_URL}/{id}.png") };
            let (small_image, small_text) = small(ctx, &id);
            let a = json!({
                "type": 0,
                "details": clamp(&details),
                "timestamps": { "start": ctx.timer_start() },
                "assets": {
                    "large_image": image,
                    "large_text": clamp(&name),
                    "small_image": small_image,
                    "small_text": clamp(&small_text),
                },
            });
            (details, Some(finish(a, Some(state.join(" | ")), Some(&id))))
        }
    }
}

/// Small image: the nation flag (asset flag_<nation>) when uploaded, else the logo;
/// its caption: "USA · Medium tank" with show_nation.
fn small(ctx: &Ctx, id: &str) -> (String, String) {
    let logo = (ctx.logo(), GAME_NAME.to_string());
    if !ctx.cfg.show_nation {
        return logo;
    }
    let nation_key = names::info(ctx.names, id, "nation");
    let text: Vec<&str> = [
        nation_key.and_then(|n| i18n::nation(ctx.lang, n)),
        names::info(ctx.names, id, "class").and_then(|c| i18n::class(ctx.lang, c)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if text.is_empty() {
        return logo;
    }
    let image = match nation_key {
        Some(n) if i18n::nation(ctx.lang, n).is_some() => ctx.image(&format!("flag_{n}"), "flags"),
        _ => logo.0,
    };
    (image, text.join(" · "))
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
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 100, battle: &Battle::default() };
        let (status, act) = build(&tank(), &ctx);
        let act = act.unwrap();
        assert_eq!(status, "Plays on: M1A1 SA");
        assert_eq!(act["state"], "Speed: 38 km/h | Crew: 3/4");
        assert_eq!(act["assets"]["large_image"], format!("{IMAGE_URL}/us_m1a1_aim_abrams.png"));
        assert_eq!(act["assets"]["small_image"], format!("{REPO_ASSETS}/logo/main_white.png?v={}", env!("CARGO_PKG_VERSION")));
        assert_eq!(act["timestamps"]["start"], 100);
    }

    #[test]
    fn air_presence_ru_with_custom_image() {
        let cfg = Config { air_speed: AirSpeed::Tas, ..Default::default() };
        let n = names();
        let a = HashSet::from(["su_27".to_string()]);
        let ctx = Ctx { cfg: &cfg, lang: Lang::Ru, names: &n, assets: &a, start: 0, battle: &Battle::default() };
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
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 0, battle: &Battle::default() };
        assert_eq!(build(&plane(), &ctx).1.unwrap()["state"], "IAS: 612 km/h | Altitude: 3050 m");
        let cfg = Config { show_indicators: false, ..Default::default() };
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 0, battle: &Battle::default() };
        assert!(build(&plane(), &ctx).1.unwrap().get("state").is_none());
    }

    #[test]
    fn optional_extras() {
        let n = crate::names::parse(r#"{"us_m1a1_aim_abrams":{"English":"M1A1 SA","nation":"usa","class":"medium_tank"}}"#).unwrap();
        let cfg = Config {
            show_kills: true,
            nickname: "me".into(),
            show_nation: true,
            battle_timer: true,
            wiki_button: true,
            repo_button: true,
            ..Default::default()
        };
        let mut battle = Battle::default();
        battle.tick(true, 500);
        battle.on_messages(&[], "me");
        battle.on_messages(&[(1, "me (M1A1 SA) destroyed X (T-90M)".into())], "me");
        let a = HashSet::from(["flag_usa".to_string()]);
        let ctx = Ctx { cfg: &cfg, lang: Lang::Ru, names: &n, assets: &a, start: 100, battle: &battle };
        let act = build(&tank(), &ctx).1.unwrap();
        assert_eq!(act["state"], "Скорость: 38 км/ч | Экипаж: 3/4 | Уничтожено: 1");
        assert_eq!(act["timestamps"]["start"], 500);
        assert_eq!(act["assets"]["small_image"], "flag_usa");
        assert_eq!(act["assets"]["small_text"], "США · Средний танк");
        assert_eq!(act["buttons"][0]["url"], "https://wiki.warthunder.com/unit/us_m1a1_aim_abrams");
        assert_eq!(act["buttons"].as_array().unwrap().len(), 2);
        // no flag asset uploaded: the flag from the repository
        let none = HashSet::new();
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &none, start: 100, battle: &battle };
        let act = build(&tank(), &ctx).1.unwrap();
        assert_eq!(act["assets"]["small_image"], format!("{REPO_ASSETS}/flags/flag_usa.png?v={}", env!("CARGO_PKG_VERSION")));
        assert_eq!(act["assets"]["small_text"], "USA · Medium tank");
        // in the hangar: session timer, no kills
        battle.tick(false, 900);
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &none, start: 100, battle: &battle };
        let act = build(&Snapshot::Hangar, &ctx).1.unwrap();
        assert_eq!(act["timestamps"]["start"], 100);
        assert!(act.get("state").is_none());
        assert_eq!(act["buttons"].as_array().unwrap().len(), 1);
        // only the wiki button: none in the hangar
        let cfg = Config { wiki_button: true, ..Default::default() };
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &none, start: 100, battle: &battle };
        assert!(build(&Snapshot::Hangar, &ctx).1.unwrap().get("buttons").is_none());
        let act = build(&tank(), &ctx).1.unwrap();
        assert_eq!(act["buttons"].as_array().unwrap().len(), 1);
        assert_eq!(act["buttons"][0]["label"], "Vehicle on wiki");
    }

    #[test]
    fn extras_are_off_by_default() {
        let (cfg, n, a) = (Config::default(), names(), HashSet::from(["flag_usa".to_string()]));
        let mut battle = Battle::default();
        battle.tick(true, 500);
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 100, battle: &battle };
        let act = build(&tank(), &ctx).1.unwrap();
        assert_eq!(act["timestamps"]["start"], 100);
        assert_eq!(act["assets"]["small_image"], format!("{REPO_ASSETS}/logo/main_white.png?v={}", env!("CARGO_PKG_VERSION")));
        assert_eq!(act["assets"]["small_text"], GAME_NAME);
        assert!(act.get("buttons").is_none());
        assert_eq!(act["state"], "Speed: 38 km/h | Crew: 3/4");
    }

    #[test]
    fn uploaded_logo_wins_over_repository() {
        let (cfg, n) = (Config { logo: crate::config::Logo::Red, ..Default::default() }, names());
        let a = HashSet::from(["main_red".to_string()]);
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &a, start: 0, battle: &Battle::default() };
        assert_eq!(build(&Snapshot::Hangar, &ctx).1.unwrap()["assets"]["large_image"], "main_red");
        let none = HashSet::new();
        let ctx = Ctx { cfg: &cfg, lang: Lang::En, names: &n, assets: &none, start: 0, battle: &Battle::default() };
        assert_eq!(build(&Snapshot::Hangar, &ctx).1.unwrap()["assets"]["large_image"], format!("{REPO_ASSETS}/logo/main_red.png?v={}", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn hangar_and_offline() {
        let (cfg, n, a) = (Config::default(), names(), HashSet::new());
        let ctx = Ctx { cfg: &cfg, lang: Lang::Ru, names: &n, assets: &a, start: 0, battle: &Battle::default() };
        assert_eq!(build(&Snapshot::Hangar, &ctx).1.unwrap()["details"], "В ангаре");
        assert!(build(&Snapshot::NotRunning, &ctx).1.is_none());
    }
}
