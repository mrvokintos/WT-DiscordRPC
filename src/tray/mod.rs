// Platform-neutral tray menu: both backends render the same list of items.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

use crate::app::App;
use crate::config::{AirSpeed, Altitude, Lang, Logo};
use crate::i18n::tr;
use crate::{autostart, paths, update};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    ToggleIndicators,
    ToggleCustomImages,
    ToggleKills,
    ToggleNation,
    ToggleBattleTimer,
    ToggleButtons,
    Logo(Logo),
    Lang(Lang),
    AirSpeed(AirSpeed),
    Altitude(Altitude),
    ToggleAutostart,
    OpenConfig,
    UpdateNames,
    OpenReleases,
    Quit,
}

pub enum Item {
    Label(String),
    Action(String, Action),
    Check(String, bool, Action),
    /// submenu label, selected index, options
    Radio(String, usize, Vec<(String, Action)>),
    /// plain submenu
    Sub(String, Vec<Item>),
    Separator,
}

pub struct Menu;

pub fn status_line(app: &App) -> String {
    app.with(|s| {
        let l = s.cfg.lang();
        if !s.discord_ok && s.status != tr(l, "game_not_running") && !s.status.is_empty() {
            format!("{}\n{}", s.status, tr(l, "discord_not_running"))
        } else {
            s.status.clone()
        }
    })
}

impl Menu {
    pub fn build(app: &App) -> Vec<Item> {
        let status = status_line(app);
        let autostart = autostart::is_enabled();
        app.with(|s| {
            let c = &s.cfg;
            let l = c.lang();
            let mut items = vec![];
            for line in status.lines().filter(|x| !x.is_empty()) {
                items.push(Item::Label(line.to_string()));
            }
            if let Some(v) = &s.new_version {
                items.push(Item::Action(format!("{}: {v}", tr(l, "new_version")), Action::OpenReleases));
            }
            items.push(Item::Separator);
            items.push(Item::Check(tr(l, "show_indicators").into(), c.show_indicators, Action::ToggleIndicators));
            items.push(Item::Radio(
                tr(l, "air_speed").into(),
                (c.air_speed == AirSpeed::Tas) as usize,
                vec![("IAS".into(), Action::AirSpeed(AirSpeed::Ias)), ("TAS".into(), Action::AirSpeed(AirSpeed::Tas))],
            ));
            items.push(Item::Radio(
                tr(l, "altitude").into(),
                (c.altitude == Altitude::Absolute) as usize,
                vec![
                    (tr(l, "alt_radio").into(), Action::Altitude(Altitude::Radio)),
                    (tr(l, "alt_abs").into(), Action::Altitude(Altitude::Absolute)),
                ],
            ));
            items.push(Item::Check(tr(l, "custom_images").into(), c.custom_images, Action::ToggleCustomImages));
            let kills_label = if c.nickname.trim().is_empty() { "show_kills_no_nick" } else { "show_kills" };
            items.push(Item::Sub(
                tr(l, "extras").into(),
                vec![
                    Item::Check(tr(l, kills_label).into(), c.show_kills, Action::ToggleKills),
                    Item::Check(tr(l, "show_nation").into(), c.show_nation, Action::ToggleNation),
                    Item::Check(tr(l, "battle_timer").into(), c.battle_timer, Action::ToggleBattleTimer),
                    Item::Check(tr(l, "buttons").into(), c.buttons, Action::ToggleButtons),
                ],
            ));
            items.push(Item::Radio(
                tr(l, "logo").into(),
                match c.logo {
                    Logo::Red => 0,
                    Logo::White => 1,
                    Logo::Diamond => 2,
                },
                vec![
                    (tr(l, "logo_red").into(), Action::Logo(Logo::Red)),
                    (tr(l, "logo_white").into(), Action::Logo(Logo::White)),
                    (tr(l, "logo_diamond").into(), Action::Logo(Logo::Diamond)),
                ],
            ));
            items.push(Item::Radio(
                tr(l, "language").into(),
                match c.lang {
                    Lang::Auto => 0,
                    Lang::Ru => 1,
                    Lang::En => 2,
                },
                vec![
                    (tr(l, "lang_auto").into(), Action::Lang(Lang::Auto)),
                    ("Русский".into(), Action::Lang(Lang::Ru)),
                    ("English".into(), Action::Lang(Lang::En)),
                ],
            ));
            items.push(Item::Separator);
            items.push(Item::Check(tr(l, "autostart").into(), autostart, Action::ToggleAutostart));
            items.push(Item::Action(tr(l, "update_names").into(), Action::UpdateNames));
            items.push(Item::Action(tr(l, "open_config").into(), Action::OpenConfig));
            items.push(Item::Separator);
            items.push(Item::Action(tr(l, "quit").into(), Action::Quit));
            items
        })
    }
}

pub fn run(app: &App, action: Action) {
    match action {
        Action::ToggleIndicators => app.change(|c| c.show_indicators = !c.show_indicators),
        Action::ToggleCustomImages => app.change(|c| c.custom_images = !c.custom_images),
        Action::ToggleKills => app.change(|c| c.show_kills = !c.show_kills),
        Action::ToggleNation => app.change(|c| c.show_nation = !c.show_nation),
        Action::ToggleBattleTimer => app.change(|c| c.battle_timer = !c.battle_timer),
        Action::ToggleButtons => app.change(|c| c.buttons = !c.buttons),
        Action::Logo(v) => app.change(|c| c.logo = v),
        Action::Lang(v) => app.change(|c| c.lang = v),
        Action::AirSpeed(v) => app.change(|c| c.air_speed = v),
        Action::Altitude(v) => app.change(|c| c.altitude = v),
        Action::ToggleAutostart => {
            if let Err(e) = autostart::set(!autostart::is_enabled()) {
                log::error!("autostart: {e}");
            }
        }
        Action::OpenConfig => paths::open(paths::base_dir()),
        Action::UpdateNames => {
            let url = app.with(|s| s.cfg.names_url.clone());
            app.spawn_names_update(url, true);
        }
        Action::OpenReleases => paths::open(std::path::Path::new(&update::releases_page())),
        Action::Quit => app.request_quit(),
    }
}

/// Runs the tray on the current thread until quit.
pub fn run_tray(app: App) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return linux::run(app);
    #[cfg(windows)]
    return windows::run(app);
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = app;
        Err("no tray on this platform".into())
    }
}
