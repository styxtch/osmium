pub mod commands;
pub mod core;
pub mod db;
pub mod error;
pub mod logging;
pub mod spike;
pub mod state;

use crate::core::engine::concurrency::InferenceCoordinator;
use crate::core::engine::sidecar::SidecarManager;
use crate::core::engine::telemetry;
use crate::db::Database;
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

fn resolve_app_data_dir(app: &tauri::App) -> PathBuf {
    app.path()
        .app_local_data_dir()
        .unwrap_or_else(|_| {
            let base = std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| std::env::temp_dir());
            base.join("Osmium")
        })
}

fn resolve_default_export_dir(app: &tauri::App, app_data_dir: &PathBuf) -> PathBuf {
    app.path()
        .download_dir()
        .unwrap_or_else(|_| app_data_dir.join("exports"))
}

fn resolve_bin_dir(app: &tauri::App) -> PathBuf {
    if let Ok(res_dir) = app.path().resource_dir() {
        let bundled = res_dir.join("bin");
        if bundled.join("llama-server.exe").exists() {
            return bundled;
        }
    }

    let dev_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("bin");
    dev_bin
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let app_data_dir = resolve_app_data_dir(app);
            let log_guard = logging::init_logging(&app_data_dir.join("logs"));
            tracing::info!("Osmium backend initializing");

            let db_path = app_data_dir.join("osmium.db");
            let default_model_dir = app_data_dir.join("models");
            let default_export_dir = resolve_default_export_dir(app, &app_data_dir);
            let _ = std::fs::create_dir_all(&default_model_dir);
            let _ = std::fs::create_dir_all(&default_export_dir);

            let db = Database::open(&db_path, &default_model_dir, &default_export_dir)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            let _ = db.recover_stale_running_jobs();

            let active_model = db.get_active_model().unwrap_or(None);
            let sidecar = Arc::new(SidecarManager::new(active_model.is_some()));
            if let Some(ref model) = active_model {
                sidecar.set_standby_model(
                    Some(model.id.clone()),
                    Some(model.display_name.clone()),
                    "vulkan".to_string(),
                    4096,
                    99,
                );
            }

            let coordinator = Arc::new(InferenceCoordinator::new());
            let bin_dir = resolve_bin_dir(app);
            let state = AppState::new(db, sidecar, coordinator, bin_dir, log_guard);

            if let Some(model) = active_model {
                let state_clone = state.clone();
                tauri::async_runtime::spawn(async move {
                    crate::core::models::manager::sync_standby_for_active(&state_clone, &model)
                        .await;
                });
            }

            telemetry::spawn_telemetry_loop(app.handle().clone(), state.clone());
            app.manage(state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::hardware::scan_hardware,
            commands::hardware::get_engine_telemetry,
            commands::hardware::set_diagnostics_tab_open,
            commands::hardware::check_disk_space,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::models::list_models,
            commands::models::confirm_and_start_model_download,
            commands::models::pause_model_download,
            commands::models::resume_model_download,
            commands::models::cancel_model_download,
            commands::models::import_local_gguf,
            commands::models::relink_byom_model,
            commands::models::activate_model,
            commands::models::delete_model,
            commands::translator::translate_text,
            commands::translator::cancel_text_translation,
            commands::translator::get_word_alternatives,
            commands::translator::save_text_history,
            commands::translator::list_text_history,
            commands::translator::delete_text_history,
            commands::glossary::list_glossaries,
            commands::glossary::create_glossary,
            commands::glossary::update_glossary,
            commands::glossary::delete_glossary,
            commands::glossary::list_glossary_entries,
            commands::glossary::upsert_glossary_entry,
            commands::glossary::delete_glossary_entry,
            commands::subtitle::parse_and_create_subtitle_job,
            commands::subtitle::list_subtitle_jobs,
            commands::subtitle::get_subtitle_job,
            commands::subtitle::delete_subtitle_job,
            commands::subtitle::update_subtitle_job_options,
            commands::subtitle::get_subtitle_cues,
            commands::subtitle::start_subtitle_job,
            commands::subtitle::pause_subtitle_job,
            commands::subtitle::resume_subtitle_job,
            commands::subtitle::cancel_subtitle_job,
            commands::subtitle::update_cue_translation,
            commands::subtitle::retranslate_single_cue,
            commands::subtitle::export_subtitle_file,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Osmium Tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    let has_model = state.db.get_active_model().ok().flatten().is_some();
                    tauri::async_runtime::block_on(state.sidecar.stop(has_model));
                }
            }
        });
}
