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

    if let Ok(Some(record)) = state.db.get_latest() {
        let req: ParsedRequest = record.into();
        let tray_text = tray::format_tray_text(&req, &config.tray, None);
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

#[tauri::command]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn get_cost(state: State<AppState>) -> Result<f64, String> {
    let config = load_config();
    let since = tray::calculate_cost_since(&config.cost.time_window);
    state.db.calculate_cost(
        &since,
        &config.cost.project_whitelist,
        &config.cost.model_whitelist,
        &config.cost.model_prices,
    )
}

#[tauri::command]
pub async fn sync_prices() -> Result<Config, String> {
    let mut config = load_config();
    let synced = crate::price_sync::sync_prices(&config.cost.model_prices).await?;
    config.cost.model_prices = synced;
    config.cost.last_sync_time = Some(chrono::Utc::now().to_rfc3339());
    save_config(&config)?;
    Ok(config)
}

#[tauri::command]
pub fn delete_model_data(state: State<AppState>, model: String) -> Result<u64, String> {
    state.db.delete_by_model(&model)
}

#[tauri::command]
pub fn delete_all_data(state: State<AppState>) -> Result<u64, String> {
    state.db.delete_all()
}

#[tauri::command]
pub fn get_projects(state: State<AppState>) -> Result<Vec<String>, String> {
    state.db.get_projects()
}
