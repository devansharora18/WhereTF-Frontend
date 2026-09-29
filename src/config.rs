use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "Alt+K";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub spotlight_shortcut: String,
    /// Backend tier chosen on first run: lite | balanced | pro.
    #[serde(default)]
    pub backend_tier: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            spotlight_shortcut: DEFAULT_SHORTCUT.to_string(),
            backend_tier: None,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("wheretf").join("config.json"))
}

pub fn load() -> AppConfig {
    if let Some(path) = config_path() {
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&data) {
                    return cfg;
                }
            }
        }
    }
    AppConfig::default()
}

pub fn save(cfg: &AppConfig) {
    if let Some(path) = config_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(cfg) {
            let _ = std::fs::write(path, data);
        }
    }
}

pub fn get_shortcut() -> String {
    load().spotlight_shortcut
}

pub fn set_shortcut(shortcut: &str) {
    let mut cfg = load();
    cfg.spotlight_shortcut = shortcut.to_string();
    save(&cfg);
}

pub fn get_tier() -> Option<String> {
    load().backend_tier
}

pub fn set_tier(tier: &str) {
    let mut cfg = load();
    cfg.backend_tier = Some(tier.to_string());
    save(&cfg);
}
