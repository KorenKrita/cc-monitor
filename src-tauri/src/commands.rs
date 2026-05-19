use tauri::{AppHandle, Manager, State};
use std::sync::Arc;

use crate::config::{Config, load_config, save_config};
use crate::db::{Database, RequestRecord};
use crate::parser::ParsedRequest;
use crate::tray;

pub struct AppState {
    pub db: Arc<Database>,
}

#[tauri::command]
pub fn get_requests(state: State<AppState>, since: String, until: Option<String>, models: Option<Vec<String>>) -> Result<Vec<RequestRecord>, String> {
    state.db.query_requests(&since, until.as_deref(), models.as_deref())
}

#[tauri::command]
pub fn get_latest(state: State<AppState>) -> Result<Option<RequestRecord>, String> {
    state.db.get_latest()
}

#[tauri::command]
pub fn get_models(state: State<AppState>) -> Result<Vec<String>, String> {
    state.db.get_models()
}

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    Ok(load_config())
}

#[tauri::command]
pub fn set_config(app: AppHandle, state: State<AppState>, config: Config) -> Result<(), String> {
    save_config(&config)?;

    // Refresh tray with latest data using new config
    if let Ok(Some(record)) = state.db.get_latest() {
        let req = ParsedRequest {
            timestamp: record.timestamp,
            model: record.model,
            input_tokens: record.input_tokens,
            output_tokens: record.output_tokens,
            cache_creation_tokens: record.cache_creation_tokens,
            cache_read_tokens: record.cache_read_tokens,
            duration_ms: record.duration_ms,
        };
        let tray_text = tray::format_tray_text(&req, &config.tray);
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_title(Some(&tray_text));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn hide_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}
