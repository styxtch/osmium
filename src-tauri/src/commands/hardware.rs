use crate::core::engine::telemetry::{build_current_telemetry, EngineTelemetry};
use crate::core::hardware::disk;
use crate::core::hardware::scan_hardware as run_hardware_scan;
use crate::core::hardware::types::{DiskSpaceInfo, HardwareProfile};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn scan_hardware(state: State<'_, AppState>) -> AppResult<HardwareProfile> {
    let bin_dir = state.bin_dir.clone();
    let profile = tokio::task::spawn_blocking(move || run_hardware_scan(Some(&bin_dir)))
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))??;
    *state.hardware_cache.write().await = Some(profile.clone());
    Ok(profile)
}

#[tauri::command]
pub async fn get_engine_telemetry(state: State<'_, AppState>) -> AppResult<EngineTelemetry> {
    Ok(build_current_telemetry(&state))
}

#[tauri::command]
pub async fn set_diagnostics_tab_open(
    state: State<'_, AppState>,
    open: bool,
) -> AppResult<()> {
    state.diagnostics_open.store(open, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn check_disk_space(dir_path: String) -> AppResult<DiskSpaceInfo> {
    let path = PathBuf::from(dir_path);
    tokio::task::spawn_blocking(move || disk::check_disk_space(&path))
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?
}

#[tauri::command]
pub async fn unload_sidecar_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<EngineTelemetry> {
    state.coordinator.ensure_idle_for_admin_action().await?;
    let has_model = state.db.get_active_model()?.is_some();
    state.sidecar.stop(has_model).await;
    tracing::info!("Manually unloaded llama-server sidecar from memory");
    let telemetry = build_current_telemetry(&state);
    let _ = app.emit("engine:telemetry", telemetry.clone());
    crate::core::engine::telemetry::emit_vram_tick(&app);
    Ok(telemetry)
}
