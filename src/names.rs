// Vehicle id → readable name, from WT-Vehicles-Parser's names.json. A copy is built in;
// a newer one is downloaded from the db-latest release when the game version changes.

use crate::paths;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;

const EMBEDDED: &str = include_str!("../names.json");
const MIN_ENTRIES: usize = 1500;

pub type Names = HashMap<String, HashMap<String, String>>;

pub fn parse(text: &str) -> Result<Names, String> {
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())
}

pub fn load() -> Names {
    let embedded = parse(EMBEDDED).expect("embedded names.json");
    match std::fs::read_to_string(paths::file(paths::NAMES_FILE)).map_err(|e| e.to_string()).and_then(|t| parse(&t)) {
        Ok(n) if n.len() >= MIN_ENTRIES => {
            log::info!("vehicle names: {} (downloaded)", n.len());
            n
        }
        _ => {
            log::info!("vehicle names: {} (built in)", embedded.len());
            embedded
        }
    }
}

/// Name in the column (`English`, `Russian`), falling back to English, then a prettified id.
pub fn lookup(names: &Names, id: &str, column: &str) -> String {
    names
        .get(id)
        .and_then(|n| n.get(column).or_else(|| n.get("English")))
        .cloned()
        .unwrap_or_else(|| id.replace('_', " "))
}

fn installed_version() -> Option<String> {
    let text = std::fs::read_to_string(paths::file(paths::NAMES_META_FILE)).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("game_version").and_then(Value::as_str).map(str::to_string)
}

/// Downloads a newer names.json if the release has one. Returns the new table when updated.
pub fn update(url: &str, force: bool) -> Result<Option<Names>, String> {
    let url = url.trim_end_matches('/');
    if url.is_empty() {
        return Ok(None);
    }
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    let manifest: Value = agent
        .get(&format!("{url}/manifest.json"))
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| format!("manifest.json: {e}"))?;
    let Some(info) = manifest.get("names") else {
        return Err("manifest has no names.json yet".into());
    };
    let remote = manifest.get("game_version").and_then(Value::as_str).unwrap_or("").to_string();
    if !force && !remote.is_empty() && installed_version().as_deref() == Some(remote.as_str()) {
        log::info!("vehicle names are up to date (game {remote})");
        return Ok(None);
    }
    let file = info.get("file").and_then(Value::as_str).unwrap_or("names.json");
    let mut bytes = Vec::new();
    agent
        .get(&format!("{url}/{file}"))
        .call()
        .map_err(|e| e.to_string())?
        .into_reader()
        .take(32 << 20)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if let Some(want) = info.get("sha256").and_then(Value::as_str) {
        use sha2::{Digest, Sha256};
        let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        if got != want {
            return Err(format!("{file}: checksum mismatch"));
        }
    }
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let names = parse(&text)?;
    if names.len() < MIN_ENTRIES {
        return Err(format!("{file}: only {} entries", names.len()));
    }
    std::fs::write(paths::file(paths::NAMES_FILE), &text).map_err(|e| e.to_string())?;
    let meta = serde_json::json!({ "game_version": remote, "count": names.len() });
    std::fs::write(paths::file(paths::NAMES_META_FILE), meta.to_string()).map_err(|e| e.to_string())?;
    log::info!("vehicle names updated: {} (game {remote})", names.len());
    Ok(Some(names))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_is_valid() {
        let n = parse(EMBEDDED).unwrap();
        assert!(n.len() >= MIN_ENTRIES);
        assert_eq!(lookup(&n, "ussr_t_80ue1", "Russian"), "Т-80У-Е1");
        assert_eq!(lookup(&n, "ussr_t_80ue1", "English"), "T-80U-E1");
        // no Russian column when it equals English
        assert_eq!(lookup(&n, "germ_leopard_2a7v", "Russian"), "Leopard 2A7V");
        assert_eq!(lookup(&n, "no_such_vehicle", "English"), "no such vehicle");
    }
}
