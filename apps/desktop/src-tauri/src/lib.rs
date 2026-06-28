mod commands;
mod persist;
mod snapshotter;
mod state;

use std::path::PathBuf;

use tauri::Manager;

use crate::commands::*;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            // Hydrate persisted settings (if any) over defaults.
            let handle = app.handle().clone();
            if let Some(loaded) = persist::load(&handle) {
                let state = app.state::<AppState>();
                if let Ok(mut s) = state.settings.lock() {
                    *s = loaded.clone();
                }
                state.enricher.set_rdns_enabled(loaded.reverse_dns_enabled);
                if let Some(path) = loaded.geoip_db_path.as_ref() {
                    let _ = state.enricher.load_geoip(PathBuf::from(path));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_interfaces,
            start_capture,
            stop_capture,
            pause_capture,
            resume_capture,
            capture_status,
            get_settings,
            save_settings,
            export_session,
            enrichment_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
