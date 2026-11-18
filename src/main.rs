#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod api;
mod app;
mod autostart;
mod battle;
mod config;
mod discord;
mod i18n;
mod killfeed;
mod names;
mod paths;
mod presence;
mod tray;
mod update;

use app::App;
use config::Config;

const HELP: &str = "\
WT Discord RPC — Discord Rich Presence for War Thunder

Usage: wt-discord-rpc [options]
  --no-tray        run without the tray icon (Ctrl+C to quit)
  --print-config   print the config path and contents, then exit
  --version        print the version
  -h, --help       this help

Environment: WT_API_URL (game API, default http://localhost:8111),
             WT_RPC_DIR (config folder), RUST_LOG (log level, default info)
";

fn init_log() {
    let mut b = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    // a windowed Windows build has no console: log to a file next to the config
    if cfg!(all(windows, not(debug_assertions))) {
        if let Ok(f) = std::fs::File::create(paths::file(paths::LOG_FILE)) {
            b.target(env_logger::Target::Pipe(Box::new(f)));
        }
    }
    b.init();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |a: &str| args.iter().any(|x| x == a);
    if has("-h") || has("--help") {
        print!("{HELP}");
        return;
    }
    if has("--version") {
        println!("wt-discord-rpc {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    init_log();
    let cfg = Config::load();
    if has("--print-config") {
        println!("# {}", paths::file(paths::CONFIG_FILE).display());
        print!("{}", toml::to_string_pretty(&cfg).unwrap_or_default());
        return;
    }
    log::info!("wt-discord-rpc {} (config: {})", env!("CARGO_PKG_VERSION"), paths::base_dir().display());

    let app = App::new(cfg);
    app.spawn_background_jobs();
    let poller = {
        let app = app.clone();
        std::thread::Builder::new().name("poller".into()).spawn(move || app::run_poller(app)).expect("poller thread")
    };

    let tray_result = if has("--no-tray") { Err("disabled".to_string()) } else { tray::run_tray(app.clone()) };
    if let Err(e) = tray_result {
        if e != "disabled" {
            log::warn!("tray unavailable ({e}), running without it");
        }
        let _ = poller.join();
        return;
    }
    app.request_quit();
    // give the poller a moment to clear the presence
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !poller.is_finished() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
