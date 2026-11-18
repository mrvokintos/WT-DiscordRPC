// "Start with system": ~/.config/autostart/*.desktop on Linux, HKCU\...\Run on Windows.

use std::path::PathBuf;

/// The file to launch: the AppImage itself when running from one (its mount point changes every run).
fn target() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("APPIMAGE").map(PathBuf::from).filter(|p| p.is_file()) {
        return Ok(p);
    }
    std::env::current_exe().and_then(|p| p.canonicalize()).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn desktop_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("wt-discord-rpc.desktop"))
}

#[cfg(not(windows))]
pub fn is_enabled() -> bool {
    desktop_file().is_some_and(|p| p.is_file())
}

#[cfg(not(windows))]
pub fn set(enabled: bool) -> Result<(), String> {
    let path = desktop_file().ok_or("HOME is not set")?;
    if !enabled {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    let exe = target()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=WT Discord RPC\n\
         Comment=Discord Rich Presence for War Thunder\n\
         Exec={}\n\
         Icon=applications-games\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        desktop_quote(&exe.display().to_string())
    );
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Exec= quoting from the Desktop Entry spec.
#[cfg_attr(windows, allow(dead_code))]
fn desktop_quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '%' => out.push_str("%%"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::System::Registry::*;

    pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    pub const VALUE: &str = "WT Discord RPC";

    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn open(write: bool) -> Option<HKEY> {
        let mut key: HKEY = std::ptr::null_mut();
        let access = if write { KEY_SET_VALUE | KEY_QUERY_VALUE } else { KEY_QUERY_VALUE };
        let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), 0, access, &mut key) };
        (r == 0).then_some(key)
    }
}

#[cfg(windows)]
pub fn is_enabled() -> bool {
    use windows_sys::Win32::System::Registry::*;
    let Some(key) = win::open(false) else { return false };
    let r = unsafe {
        RegQueryValueExW(key, win::wide(win::VALUE).as_ptr(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
    };
    unsafe { RegCloseKey(key) };
    r == 0
}

#[cfg(windows)]
pub fn set(enabled: bool) -> Result<(), String> {
    use windows_sys::Win32::System::Registry::*;
    let key = win::open(true).ok_or("cannot open the Run registry key")?;
    let name = win::wide(win::VALUE);
    let r = if enabled {
        let exe = target()?;
        let exe = exe.display().to_string();
        let cmd = win::wide(&format!("\"{}\"", exe.trim_start_matches(r"\\?\")));
        unsafe { RegSetValueExW(key, name.as_ptr(), 0, REG_SZ, cmd.as_ptr().cast(), (cmd.len() * 2) as u32) }
    } else {
        let r = unsafe { RegDeleteValueW(key, name.as_ptr()) };
        if r == 2 { 0 } else { r } // ERROR_FILE_NOT_FOUND: already off
    };
    unsafe { RegCloseKey(key) };
    if r == 0 {
        Ok(())
    } else {
        Err(format!("registry error {r}"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_exec_is_quoted() {
        assert_eq!(super::desktop_quote("/opt/WT RPC/wt-discord-rpc"), "\"/opt/WT RPC/wt-discord-rpc\"");
        assert_eq!(super::desktop_quote(r#"/a"b$c\d%e"#), r#""/a\"b\$c\\d%%e""#);
    }
}
