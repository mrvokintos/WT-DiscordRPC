// New version check against GitHub releases. No self-update: the tray just shows the version.

use serde_json::Value;
use std::time::Duration;

pub const REPO: &str = "mrvokintos/WT-DiscordRPC";

pub fn releases_page() -> String {
    format!("https://github.com/{REPO}/releases/latest")
}

pub fn check() -> Option<String> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build();
    let v: Value = agent
        .get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .set("User-Agent", "wt-discord-rpc")
        .call()
        .map_err(|e| log::debug!("update check: {e}"))
        .ok()?
        .into_json()
        .ok()?;
    let tag = v.get("tag_name").and_then(Value::as_str)?.to_string();
    if is_newer(&tag, env!("CARGO_PKG_VERSION")) {
        log::info!("new version available: {tag}");
        Some(tag)
    } else {
        None
    }
}

fn parse(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split(['.', '-']).map_while(|p| p.parse().ok()).collect()
}

pub fn is_newer(remote: &str, local: &str) -> bool {
    parse(remote) > parse(local)
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions() {
        assert!(super::is_newer("v0.2.0", "0.1.9"));
        assert!(super::is_newer("1.0.0", "0.10.0"));
        assert!(!super::is_newer("v0.1.0", "0.1.0"));
        assert!(!super::is_newer("garbage", "0.1.0"));
    }
}
