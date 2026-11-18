// Shared state between the poller thread and the tray, and the poll loop itself.

use crate::api::{Api, Snapshot};
use crate::config::Config;
use crate::discord::{self, ipc};
use crate::names::{self, Names};
use crate::presence::{self, Ctx};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct Shared {
    pub cfg: Config,
    pub status: String,
    pub discord_ok: bool,
    /// newer release tag, if any
    pub new_version: Option<String>,
    pub names: Arc<Names>,
    pub assets: Arc<HashSet<String>>,
    pub quit: bool,
    /// set by the tray to re-send the activity right away (settings changed)
    pub dirty: bool,
}

#[derive(Clone)]
pub struct App(Arc<(Mutex<Shared>, Condvar)>);

impl App {
    pub fn new(cfg: Config) -> Self {
        let s = Shared {
            cfg,
            status: String::new(),
            discord_ok: false,
            new_version: None,
            names: Arc::new(names::load()),
            assets: Arc::new(HashSet::new()),
            quit: false,
            dirty: false,
        };
        App(Arc::new((Mutex::new(s), Condvar::new())))
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut Shared) -> R) -> R {
        f(&mut self.0 .0.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// Changes settings, saves them and wakes the poller.
    pub fn change(&self, f: impl FnOnce(&mut Config)) {
        self.with(|s| {
            f(&mut s.cfg);
            s.cfg.save();
            s.dirty = true;
        });
        self.0 .1.notify_all();
    }

    pub fn request_quit(&self) {
        self.with(|s| s.quit = true);
        self.0 .1.notify_all();
    }

    /// Sleeps up to `d`, waking early on settings change or quit.
    fn wait(&self, d: Duration) {
        let (m, cv) = &*self.0;
        let g = m.lock().unwrap_or_else(|e| e.into_inner());
        let _ = cv.wait_timeout_while(g, d, |s| !s.quit && !s.dirty);
    }

    pub fn spawn_background_jobs(&self) {
        let (names_url, check_updates, client_id) =
            self.with(|s| (s.cfg.names_url.clone(), s.cfg.check_updates, s.cfg.client_id.clone()));
        self.spawn_names_update(names_url, false);
        let app = self.clone();
        std::thread::spawn(move || match discord::fetch_asset_names(&client_id) {
            Ok(a) => {
                log::info!("Discord application images: {}", a.len());
                app.with(|s| {
                    s.assets = Arc::new(a);
                    s.dirty = true;
                });
            }
            Err(e) => log::warn!("Discord application images: {e}"),
        });
        if check_updates {
            let app = self.clone();
            std::thread::spawn(move || {
                if let Some(v) = crate::update::check() {
                    app.with(|s| s.new_version = Some(v));
                }
            });
        }
    }

    pub fn spawn_names_update(&self, url: String, force: bool) {
        let app = self.clone();
        std::thread::spawn(move || match names::update(&url, force) {
            Ok(Some(n)) => app.with(|s| {
                s.names = Arc::new(n);
                s.dirty = true;
            }),
            Ok(None) => {}
            Err(e) => log::warn!("vehicle names update: {e}"),
        });
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Discord allows about 5 activity updates per 20 s; stay well below.
const MIN_SEND_INTERVAL: Duration = Duration::from_secs(4);

pub fn run_poller(app: App) {
    let mut client: Option<ipc::Client> = None;
    let mut client_id = String::new();
    let mut next_connect = Instant::now();
    let mut sent: Option<Option<Value>> = None;
    let mut last_send = Instant::now() - MIN_SEND_INTERVAL;
    let mut start: Option<u64> = None;

    loop {
        let (cfg, names, assets, quit) = app.with(|s| {
            s.dirty = false;
            (s.cfg.clone(), s.names.clone(), s.assets.clone(), s.quit)
        });
        if quit {
            if let Some(mut c) = client.take() {
                let _ = c.set_activity(None);
                c.close();
            }
            return;
        }
        if cfg.client_id != client_id {
            client = None;
            sent = None;
            client_id = cfg.client_id.clone();
        }

        let snap = Api::new(cfg.api_url()).snapshot(cfg.show_indicators);
        match (&snap, start) {
            (Snapshot::NotRunning, _) => start = None,
            (_, None) => start = Some(now()),
            _ => {}
        }
        let ctx = Ctx { cfg: &cfg, lang: cfg.lang(), names: &names, assets: &assets, start: start.unwrap_or_else(now) };
        let (status, activity) = presence::build(&snap, &ctx);

        // only talk to Discord while the game runs, so the presence vanishes with it
        let want_client = activity.is_some() || sent.as_ref().is_some_and(Option::is_some);
        if client.is_none() && want_client && Instant::now() >= next_connect {
            match ipc::Client::connect(&client_id) {
                Ok(c) => {
                    client = Some(c);
                    sent = None;
                }
                Err(e) => {
                    log::info!("Discord is not available: {e}");
                    next_connect = Instant::now() + Duration::from_secs(15);
                }
            }
        }

        if let Some(c) = client.as_mut() {
            if sent.as_ref() != Some(&activity) && last_send.elapsed() >= MIN_SEND_INTERVAL {
                last_send = Instant::now();
                match c.set_activity(activity.as_ref()) {
                    Ok(()) => {
                        log::info!("presence: {status}");
                        sent = Some(activity.clone());
                    }
                    Err(e) => {
                        log::warn!("Discord connection lost: {e}");
                        client = None;
                        sent = None;
                        next_connect = Instant::now() + Duration::from_secs(5);
                    }
                }
            }
            if activity.is_none() && sent == Some(None) {
                // game closed and presence cleared: let go of Discord
                if let Some(c) = client.take() {
                    c.close();
                }
                sent = None;
            }
        }

        let discord_ok = client.is_some();
        app.with(|s| {
            s.status = status;
            s.discord_ok = discord_ok;
        });
        app.wait(Duration::from_secs(cfg.interval_secs));
    }
}
