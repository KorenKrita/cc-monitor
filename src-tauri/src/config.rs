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
    #[serde(default = "default_time_window_value")]
    pub time_window_value: u32,
    #[serde(default)]
    pub project_whitelist: Vec<String>,
    #[serde(default)]
    pub model_whitelist: Vec<String>,
    #[serde(default = "default_model_prices")]
    pub model_prices: HashMap<String, ModelPrice>,
    #[serde(default)]
    pub last_sync_time: Option<String>,
    #[serde(default = "default_watch_sources")]
    pub watch_sources: Vec<String>,
    #[serde(default = "default_sync_source")]
    pub sync_source: String,
}

fn default_time_window() -> String { "day".into() }
fn default_time_window_value() -> u32 { 1 }
fn default_watch_sources() -> Vec<String> { vec!["claude".into(), "codex".into()] }
fn default_sync_source() -> String { "all".into() }

fn default_model_prices() -> HashMap<String, ModelPrice> {
    let mut prices = HashMap::new();
    let models = [
        ("claude-opus-4-7", 15.0, 75.0, 1.88),
        ("claude-sonnet-4-6", 3.0, 15.0, 0.3),
        ("claude-haiku-4-5-20251001", 0.8, 4.0, 0.08),
        ("gpt-4o", 2.5, 10.0, 1.25),
        ("gpt-4o-mini", 0.15, 0.6, 0.075),
        ("o3", 10.0, 40.0, 2.5),
        ("o4-mini", 1.1, 4.4, 0.275),
        ("gemini-2.5-pro", 1.25, 10.0, 0.315),
        ("gemini-2.5-flash", 0.15, 0.6, 0.0375),
        ("deepseek-chat", 0.27, 1.1, 0.07),
        ("deepseek-reasoner", 0.55, 2.19, 0.14),
    ];
    for (name, input, output, cache) in models {
        prices.insert(name.to_string(), ModelPrice {
            input, output, cache, source: "bundled".to_string(),
        });
    }
    prices
}

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            time_window: default_time_window(),
            time_window_value: default_time_window_value(),
            project_whitelist: vec![],
            model_whitelist: vec![],
            model_prices: default_model_prices(),
            last_sync_time: None,
            watch_sources: default_watch_sources(),
            sync_source: default_sync_source(),
        }
    }
}

impl CostConfig {
    pub fn cost_since(&self) -> String {
        let v = self.time_window_value.max(1) as i64;
        let now = chrono::Utc::now();
        match self.time_window.as_str() {
            "day" => (now - chrono::Duration::days(v)).to_rfc3339(),
            "month" => (now - chrono::Duration::days(v * 30)).to_rfc3339(),
            "year" => (now - chrono::Duration::days(v * 365)).to_rfc3339(),
            "all" => "1970-01-01T00:00:00Z".to_string(),
            _ => (now - chrono::Duration::days(v)).to_rfc3339(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_cost() {
        let config = Config::default();
        assert_eq!(config.cost.time_window, "day");
        assert_eq!(config.cost.watch_sources, vec!["claude", "codex"]);
        assert_eq!(config.cost.sync_source, "all");
        assert!(!config.cost.model_prices.is_empty());
        assert!(config.cost.model_prices.contains_key("claude-opus-4-7"));
        assert_eq!(config.cost.model_prices["claude-opus-4-7"].source, "bundled");
        assert!(config.cost.project_whitelist.is_empty());
        assert!(config.cost.last_sync_time.is_none());
    }

    #[test]
    fn test_deserialize_without_cost_field() {
        let json = r#"{"theme":"dark","tray":{"items":["out_rate"]},"model_aliases":{}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.cost.time_window, "day");
        assert_eq!(config.cost.watch_sources, vec!["claude", "codex"]);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let mut config = Config::default();
        config.cost.model_prices.insert("claude-opus-4-7".to_string(), ModelPrice {
            input: 15.0,
            output: 75.0,
            cache: 1.88,
            source: "manual".to_string(),
        });
        config.cost.time_window = "month".to_string();

        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cost.time_window, "month");
        assert_eq!(parsed.cost.model_prices["claude-opus-4-7"].input, 15.0);
        assert_eq!(parsed.cost.model_prices["claude-opus-4-7"].source, "manual");
    }

    #[test]
    fn test_model_price_defaults() {
        let json = r#"{"input": 10.0, "output": 30.0}"#;
        let price: ModelPrice = serde_json::from_str(json).unwrap();
        assert_eq!(price.cache, 0.0);
        assert_eq!(price.source, "manual");
    }

    #[test]
    fn test_cost_config_with_whitelist() {
        let json = r#"{"time_window":"year","project_whitelist":["-Users-test"],"model_whitelist":["claude-opus-4-7"],"model_prices":{},"watch_sources":["claude"]}"#;
        let cost: CostConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cost.time_window, "year");
        assert_eq!(cost.project_whitelist, vec!["-Users-test"]);
        assert_eq!(cost.model_whitelist, vec!["claude-opus-4-7"]);
        assert_eq!(cost.watch_sources, vec!["claude"]);
    }
}
