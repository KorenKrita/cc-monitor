use tauri::State;
use std::sync::Arc;

use crate::config::{Config, load_config, save_config};
use crate::db::{Database, RequestRecord};

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
pub fn set_config(config: Config) -> Result<(), String> {
    save_config(&config)
}

#[tauri::command]
pub fn hide_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}
