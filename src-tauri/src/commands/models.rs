use crate::core::engine::telemetry::build_current_telemetry;
use crate::core::models::catalog::find_catalog_model;
use crate::core::models::downloader::DownloadProgressEvent;
use crate::core::models::manager::{
    self, InstalledModelWithHealth, ModelsOverview,
};
use crate::db::models::DownloadTask;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> AppResult<ModelsOverview> {
    manager::get_models_overview(&state.db)
}

#[tauri::command]
pub async fn confirm_and_start_model_download(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
    target_dir: String,
    on_progress: Channel<DownloadProgressEvent>,
) -> AppResult<DownloadTask> {
    let catalog_model = find_catalog_model(&model_id)
        .ok_or_else(|| AppError::NotFound(format!("Catalog model '{model_id}'")))?;
    let dir = PathBuf::from(&target_dir);

    let task = state
        .downloads
        .start_or_resume(&state.db, &catalog_model, &dir, move |evt| {
            let _ = on_progress.send(evt);
        })
        .await?;

    if task.status == "completed" {
        if let Ok(Some(active)) = state.db.get_active_model() {
            manager::sync_standby_for_active(&state, &active).await;
            let _ = app.emit("engine:telemetry", build_current_telemetry(&state));
        }
    }

    Ok(task)
}

#[tauri::command]
pub async fn pause_model_download(
    state: State<'_, AppState>,
    model_id: String,
) -> AppResult<()> {
    let paused = state.downloads.pause(&model_id).await;
    if !paused {
        if let Some(mut task) = state.db.get_download_task(&model_id)? {
            task.status = "paused".to_string();
            state.db.upsert_download_task(&task)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn resume_model_download(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
    on_progress: Channel<DownloadProgressEvent>,
) -> AppResult<DownloadTask> {
    let catalog_model = find_catalog_model(&model_id)
        .ok_or_else(|| AppError::NotFound(format!("Catalog model '{model_id}'")))?;

    let target_dir = if let Some(existing) = state.db.get_download_task(&model_id)? {
        Path::new(&existing.target_path)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(state.db.get_settings().map(|s| s.default_model_dir).unwrap_or_default()))
    } else {
        PathBuf::from(state.db.get_settings()?.default_model_dir)
    };

    let task = state
        .downloads
        .start_or_resume(&state.db, &catalog_model, &target_dir, move |evt| {
            let _ = on_progress.send(evt);
        })
        .await?;

    if task.status == "completed" {
        if let Ok(Some(active)) = state.db.get_active_model() {
            manager::sync_standby_for_active(&state, &active).await;
            let _ = app.emit("engine:telemetry", build_current_telemetry(&state));
        }
    }

    Ok(task)
}

#[tauri::command]
pub async fn cancel_model_download(
    state: State<'_, AppState>,
    model_id: String,
) -> AppResult<()> {
    state.downloads.cancel(&model_id, &state.db).await
}

#[tauri::command]
pub async fn import_local_gguf(
    app: AppHandle,
    state: State<'_, AppState>,
    file_path: String,
) -> AppResult<InstalledModelWithHealth> {
    let imported = manager::import_local_gguf(&state, &file_path).await?;
    let _ = app.emit("engine:telemetry", build_current_telemetry(&state));
    Ok(imported)
}

#[tauri::command]
pub async fn relink_byom_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
    new_file_path: String,
) -> AppResult<InstalledModelWithHealth> {
    let updated = manager::relink_byom_model(&state, &model_id, &new_file_path).await?;
    let _ = app.emit("engine:telemetry", build_current_telemetry(&state));
    Ok(updated)
}

#[tauri::command]
pub async fn activate_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> AppResult<InstalledModelWithHealth> {
    let activated = manager::activate_model(&state, &model_id).await?;
    let _ = app.emit("engine:telemetry", build_current_telemetry(&state));
    Ok(activated)
}

#[tauri::command]
pub async fn delete_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> AppResult<()> {
    manager::delete_model(&state, &model_id).await?;
    let _ = app.emit("engine:telemetry", build_current_telemetry(&state));
    Ok(())
}
