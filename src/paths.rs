// Where config and downloaded data live: next to the executable when a config.toml is
// already there (portable mode), otherwise the user's config folder.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const CONFIG_FILE: &str = "config.toml";
pub const NAMES_FILE: &str = "names.json";
pub const NAMES_META_FILE: &str = "names.meta.json";
pub const LOG_FILE: &str = "wt-discord-rpc.log";

static BASE: OnceLock<PathBuf> = OnceLock::new();

pub fn base_dir() -> &'static Path {
    BASE.get_or_init(detect)
}

pub fn file(name: &str) -> PathBuf {
    base_dir().join(name)
}

fn detect() -> PathBuf {
    if let Some(dir) = std::env::var_os("WT_RPC_DIR").map(PathBuf::from) {
        let _ = std::fs::create_dir_all(&dir);
        return dir;
    }
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf));
    if let Some(d) = exe_dir.filter(|d| d.join(CONFIG_FILE).is_file()) {
        return d;
    }
    let dir = user_dir().unwrap_or_else(|| PathBuf::from("."));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn user_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("WT Discord RPC"))
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("wt-discord-rpc"))
    }
}

/// Opens a file or folder with the desktop's default handler.
pub fn open(path: &Path) {
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(path).spawn();
    if let Err(e) = r {
        log::warn!("cannot open {}: {e}", path.display());
    }
}
