use crate::paths;
use serde::{Deserialize, Serialize};

/// Discord application whose name ("War Thunder") and assets (`main_red`, `main_white`,
/// optional per-vehicle images named by vehicle id) the presence uses.
pub const DEFAULT_CLIENT_ID: &str = "1572641454017482922";
pub const DEFAULT_API_URL: &str = "http://localhost:8111";
pub const DEFAULT_NAMES_URL: &str = "https://github.com/mrvokintos/WT-Vehicles-Parser/releases/download/db-latest";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Auto,
    Ru,
    En,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Logo {
    #[serde(rename = "main_red")]
    Red,
    #[serde(rename = "main_white")]
    White,
}

impl Logo {
    pub fn asset(self) -> &'static str {
        match self {
            Logo::Red => "main_red",
            Logo::White => "main_white",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AirSpeed {
    Ias,
    Tas,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Altitude {
    Radio,
    Absolute,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Discord application id
    pub client_id: String,
    /// interface and vehicle name language: auto, ru, en
    pub lang: Lang,
    /// game logo: main_red or main_white
    pub logo: Logo,
    /// speed / altitude / crew in the second line
    pub show_indicators: bool,
    /// aircraft speed: IAS or TAS
    pub air_speed: AirSpeed,
    /// aircraft altitude: RADIO (falls back to ABSOLUTE when unavailable) or ABSOLUTE
    pub altitude: Altitude,
    /// use the application's own images for vehicles that have one
    pub custom_images: bool,
    /// game API poll interval, seconds
    pub interval_secs: u64,
    /// game API (the WT_API_URL environment variable overrides it)
    pub api_url: String,
    /// where names.json and manifest.json are downloaded from; empty disables updates
    pub names_url: String,
    /// check GitHub for a new version of the program
    pub check_updates: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client_id: DEFAULT_CLIENT_ID.into(),
            lang: Lang::Auto,
            logo: Logo::White,
            show_indicators: true,
            air_speed: AirSpeed::Ias,
            altitude: Altitude::Radio,
            custom_images: true,
            interval_secs: 5,
            api_url: DEFAULT_API_URL.into(),
            names_url: DEFAULT_NAMES_URL.into(),
            check_updates: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = paths::file(paths::CONFIG_FILE);
        let cfg = match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("{}: {e}; using defaults", path.display());
                    return Self::default();
                }
            },
            Err(_) => {
                let c = Self::default();
                c.save();
                c
            }
        };
        cfg.sanitized()
    }

    fn sanitized(mut self) -> Self {
        self.interval_secs = self.interval_secs.clamp(1, 60);
        self
    }

    pub fn save(&self) {
        let path = paths::file(paths::CONFIG_FILE);
        match toml::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    log::error!("{}: {e}", path.display());
                }
            }
            Err(e) => log::error!("config: {e}"),
        }
    }

    pub fn api_url(&self) -> String {
        std::env::var("WT_API_URL").unwrap_or_else(|_| self.api_url.clone()).trim_end_matches('/').to_string()
    }

    /// Resolved language: `auto` follows the system locale.
    pub fn lang(&self) -> Lang {
        match self.lang {
            Lang::Auto => {
                let sys = sys_locale::get_locale().unwrap_or_default().to_lowercase();
                if ["ru", "be", "uk", "kk"].iter().any(|l| sys.starts_with(l)) {
                    Lang::Ru
                } else {
                    Lang::En
                }
            }
            l => l,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_partial() {
        let c = Config::default();
        let text = toml::to_string_pretty(&c).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), c);
        let p: Config = toml::from_str("lang = \"ru\"\nlogo = \"main_red\"\nair_speed = \"TAS\"").unwrap();
        assert_eq!(p.lang, Lang::Ru);
        assert_eq!(p.logo, Logo::Red);
        assert_eq!(p.air_speed, AirSpeed::Tas);
        assert_eq!(p.interval_secs, 5);
    }
}
