#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{
    tray::TrayIconBuilder, Emitter, Manager, WindowEvent,
};
use tokio::sync::mpsc;

mod commands;
mod config;
mod db;
mod parser;
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
            let handle = app.handle().clone();
            let db_clone = db.clone();
            let config_clone = config.clone();

            // Create tray - show last known data or idle text
            let initial_text = match db.get_latest() {
                Ok(Some(record)) => {
                    let req = parser::ParsedRequest {
                        timestamp: record.timestamp,
                        model: record.model,
                        input_tokens: record.input_tokens,
                        output_tokens: record.output_tokens,
                        cache_creation_tokens: record.cache_creation_tokens,
                        cache_read_tokens: record.cache_read_tokens,
                        duration_ms: record.duration_ms,
                    };
                    tray::format_tray_text(&req, &config.tray)
                }
                _ => tray::format_idle_tray_text(&config.tray),
            };
            let icon = tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1);
            let _tray = TrayIconBuilder::new()
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

            // Hide window on focus lost
            let main_window = app.get_webview_window("main").unwrap();
            let win_clone = main_window.clone();
            main_window.on_window_event(move |event| {
                if let WindowEvent::Focused(false) = event {
                    let _ = win_clone.hide();
                }
            });

            // Start file watcher
            let (tx, mut rx) = mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let _watcher = watcher::FileWatcher::start(tx)
                    .expect("Failed to start file watcher");
                std::thread::park();
            });

            // Process incoming requests
            let handle_clone = handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(request) = rx.recv().await {
                    let _ = db_clone.insert_request(
                        &request.timestamp,
                        &request.model,
                        request.input_tokens,
                        request.output_tokens,
                        request.cache_creation_tokens,
                        request.cache_read_tokens,
                        request.duration_ms,
                    );

                    // Only update tray if we have valid rate data (duration > 0)
                    if request.duration_ms.filter(|&ms| ms > 0).is_some() {
                        let tray_text = tray::format_tray_text(&request, &config_clone.tray);
                        if let Some(tray) = handle_clone.tray_by_id("main") {
                            let _ = tray.set_title(Some(&tray_text));
                        }
                    }

                    let _ = handle_clone.emit("new-request", &request);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
