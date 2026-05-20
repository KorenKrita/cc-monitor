#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{
    tray::TrayIconBuilder, Emitter, Manager, WindowEvent,
};
use tokio::sync::mpsc;

mod codex_parser;
mod commands;
mod config;
mod db;
mod parser;
mod price_sync;
mod tray;
mod watcher;

use commands::AppState;

fn main() {
    let db = Arc::new(db::Database::new().expect("Failed to initialize database"));
    let config = config::load_config();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { db: db.clone() })
        .setup(move |app| {
            // Hide from dock
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                let _ = app.handle().set_activation_policy(ActivationPolicy::Accessory);
            }

            let handle = app.handle().clone();
            let db_clone = db.clone();

            // Create tray with hexagon icon + text
            let initial_cost = if config.tray.items.contains(&"cost".to_string()) {
                let since = config.cost.cost_since();
                db.calculate_cost(
                    &since,
                    &config.cost.project_whitelist,
                    &config.cost.model_whitelist,
                    &config.cost.model_prices,
                ).ok()
            } else {
                None
            };

            let initial_text = match config.tray.display_mode.as_str() {
                "average" => {
                    let mins = config.tray.average_minutes.max(1);
                    let since = chrono::Utc::now() - chrono::Duration::minutes(mins as i64);
                    tray::format_average_tray_text(&db, &since.to_rfc3339(), &config.tray, initial_cost)
                }
                _ => match db.get_latest() {
                    Ok(Some(record)) => {
                        let req: parser::ParsedRequest = record.into();
                        tray::format_tray_text(&req, &config.tray, initial_cost)
                    }
                    _ => tray::format_idle_tray_text(&config.tray, initial_cost),
                },
            };
            let icon = tauri::image::Image::new_owned(vec![0,0,0,0], 1, 1);
            let tray = TrayIconBuilder::with_id("main")
                .icon(icon)
                .icon_as_template(true)
                .show_menu_on_left_click(false)
                .title(&initial_text)
                .tooltip("CC Monitor")
                .on_tray_icon_event(move |tray_icon, event| {
                    use tauri::tray::{TrayIconEvent, MouseButton, MouseButtonState};
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let app = tray_icon.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;
            let _ = tray.set_icon(None);

            // Hide window on focus lost
            let main_window = app.get_webview_window("main").unwrap();
            let win_clone = main_window.clone();
            main_window.on_window_event(move |event| {
                if let WindowEvent::Focused(false) = event {
                    let _ = win_clone.hide();
                }
            });

            // Start file watcher (polling every 1s)
            let (tx, mut rx) = mpsc::unbounded_channel();
            watcher::start_polling(tx);

            // Process incoming requests
            let handle_clone = handle.clone();
            let app_start = chrono::Utc::now();
            tauri::async_runtime::spawn(async move {
                let mut current_config = config::load_config();
                let mut config_mtime = std::fs::metadata(config::config_dir().join("settings.json"))
                    .and_then(|m| m.modified()).ok();

                while let Some(request) = rx.recv().await {
                    // Skip old messages (from files modified but containing historical data)
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&request.timestamp) {
                        if ts < app_start - chrono::Duration::hours(1) {
                            continue;
                        }
                    }

                    let _ = db_clone.insert_request(
                        &request.timestamp,
                        &request.model,
                        request.input_tokens,
                        request.output_tokens,
                        request.cache_creation_tokens,
                        request.cache_read_tokens,
                        request.duration_ms,
                        &request.project,
                        &request.source,
                    );

                    let _ = handle_clone.emit("new-request", &request);

                    // Reload config only if file changed
                    let new_mtime = std::fs::metadata(config::config_dir().join("settings.json"))
                        .and_then(|m| m.modified()).ok();
                    if new_mtime != config_mtime {
                        current_config = config::load_config();
                        config_mtime = new_mtime;
                    }

                    let should_display = match current_config.tray.model_filter.as_str() {
                        "whitelist" => current_config.tray.model_whitelist.contains(&request.model),
                        _ => true,
                    };

                    if !should_display && current_config.tray.display_mode != "average" {
                        continue;
                    }

                    let cost = if current_config.tray.items.contains(&"cost".to_string()) {
                        let since = current_config.cost.cost_since();
                        db_clone.calculate_cost(
                            &since,
                            &current_config.cost.project_whitelist,
                            &current_config.cost.model_whitelist,
                            &current_config.cost.model_prices,
                        ).ok()
                    } else {
                        None
                    };

                    let tray_text = match current_config.tray.display_mode.as_str() {
                        "average" => {
                            let mins = current_config.tray.average_minutes.max(1);
                            let since = chrono::Utc::now() - chrono::Duration::minutes(mins as i64);
                            tray::format_average_tray_text(&db_clone, &since.to_rfc3339(), &current_config.tray, cost)
                        }
                        _ => {
                            if request.duration_ms.filter(|&ms| ms > 0).is_some() {
                                tray::format_tray_text(&request, &current_config.tray, cost)
                            } else {
                                continue;
                            }
                        }
                    };
                    if let Some(tray) = handle_clone.tray_by_id("main") {
                        let _ = tray.set_title(Some(&tray_text));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_requests,
            commands::get_latest,
            commands::get_models,
            commands::get_config,
            commands::set_config,
            commands::hide_window,
            commands::quit_app,
            commands::get_cost,
            commands::sync_prices,
            commands::delete_model_data,
            commands::delete_all_data,
            commands::get_projects,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
