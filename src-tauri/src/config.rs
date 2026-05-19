use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPrice {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default)]
    pub cache: f64,
    #[serde(default = "default_price_source")]
    pub source: String,
}

fn default_price_source() -> String { "manual".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    #[serde(default = "default_time_window")]
    pub time_window: String,
    #[serde(default)]
    pub project_whitelist: Vec<String>,
    #[serde(default)]
    pub model_whitelist: Vec<String>,
    #[serde(default)]
    pub model_prices: HashMap<String, ModelPrice>,
    #[serde(default)]
    pub last_sync_time: Option<String>,
    #[serde(default = "default_watch_sources")]
    pub watch_sources: Vec<String>,
}

fn default_time_window() -> String { "day".into() }
fn default_watch_sources() -> Vec<String> { vec!["claude".into(), "codex".into()] }

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            time_window: default_time_window(),
            project_whitelist: vec![],
            model_whitelist: vec![],
            model_prices: HashMap::new(),
            last_sync_time: None,
            watch_sources: default_watch_sources(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub tray: TrayConfig,
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,
    #[serde(default)]
    pub cost: CostConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayConfig {
    #[serde(default = "default_items")]
    pub items: Vec<String>,
    #[serde(default = "default_model_filter")]
    pub model_filter: String,
    #[serde(default)]
    pub model_whitelist: Vec<String>,
    #[serde(default = "default_display_mode")]
    pub display_mode: String,
    #[serde(default = "default_average_minutes")]
    pub average_minutes: u32,
}

fn default_theme() -> String { "system".into() }
fn default_items() -> Vec<String> { vec!["out_rate".into(), "in_rate".into(), "ttft".into()] }
fn default_model_filter() -> String { "last".into() }
fn default_display_mode() -> String { "last".into() }
fn default_average_minutes() -> u32 { 5 }

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            items: default_items(),
            model_filter: default_model_filter(),
            model_whitelist: vec![],
            display_mode: default_display_mode(),
            average_minutes: default_average_minutes(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            tray: TrayConfig::default(),
            model_aliases: HashMap::new(),
            cost: CostConfig::default(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("cc-monitor")
}

pub fn load_config() -> Config {
    let path = config_dir().join("settings.json");
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save_config(config: &Config) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(dir.join("settings.json"), content).map_err(|e| e.to_string())
}
