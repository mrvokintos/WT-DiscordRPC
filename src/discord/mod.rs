pub mod ipc;

use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;

/// Names of the images uploaded to the Discord application (public endpoint, no auth).
pub fn fetch_asset_names(client_id: &str) -> Result<HashSet<String>, String> {
    let url = format!("https://discord.com/api/v9/oauth2/applications/{client_id}/assets");
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build();
    let v: Value = agent.get(&url).call().map_err(|e| e.to_string())?.into_json().map_err(|e| e.to_string())?;
    Ok(v.as_array()
        .map(|a| a.iter().filter_map(|x| x.get("name").and_then(Value::as_str).map(str::to_string)).collect())
        .unwrap_or_default())
}
