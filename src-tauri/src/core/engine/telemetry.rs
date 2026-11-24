use crate::core::engine::concurrency::BusyState;
use crate::core::engine::sidecar::SidecarStatus;
use crate::core::hardware::dxgi;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineTelemetry {
    pub status: SidecarStatus,
    pub pid: Option<u32>,
    pub port: Option<u16>,
    pub active_model_id: Option<String>,
    pub active_model_name: Option<String>,
    pub backend: String,
    pub ctx_size: u32,
    pub gpu_layers: u32,
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub last_ttft_ms: Option<u64>,
    pub last_tok_per_sec: Option<f64>,
    pub db_wal_synced: bool,
    pub busy_by: BusyState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VramTickPayload {
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub gpu_temp_c: Option<u32>,
    pub gpu_clock_mhz: Option<u32>,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
}

pub fn build_current_telemetry(state: &AppState) -> EngineTelemetry {
    let (vram_used_mb, vram_total_mb) = dxgi::query_live_vram_mb().unwrap_or((0, 0));
    let db_wal_synced = state.db.check_wal_synced();
    let busy_by = state.coordinator.current_busy_nonblocking();
    state
        .sidecar
        .snapshot_telemetry(vram_used_mb, vram_total_mb, db_wal_synced, busy_by)
}

pub fn emit_vram_tick(app: &AppHandle) {
    let nvml = crate::core::hardware::vulkan::probe_nvml_snapshot();
    let (vram_used_mb, vram_total_mb) = match (nvml.vram_used_mb, nvml.vram_total_mb) {
        (Some(used), Some(total)) if total > 0 => (used, total),
        _ => dxgi::detect_best_dxgi_adapter()
            .ok()
            .map(|a| (a.current_vram_usage_mb, a.dedicated_vram_mb))
            .unwrap_or((0, 0)),
    };
    let (ram_used_gb, ram_total_gb) = dxgi::query_live_ram_gb().unwrap_or((0.0, 0.0));
    if vram_total_mb > 0 || ram_total_gb > 0.0 {
        let _ = app.emit(
            "engine:vram_tick",
            VramTickPayload {
                vram_used_mb,
                vram_total_mb,
                gpu_temp_c: nvml.thermal_c,
                gpu_clock_mhz: nvml.clock_mhz,
                ram_used_gb,
                ram_total_gb,
            },
        );
    }
}

pub fn spawn_telemetry_loop(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut tick_count: u64 = 0;
        let mut last_busy = BusyState::None;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            tick_count = tick_count.wrapping_add(1);

            if tick_count == 1 || tick_count % 5 == 0 {
                emit_vram_tick(&app);
            }

            if tick_count % 2 == 0 {
                let diag_open = state.diagnostics_open.load(Ordering::Relaxed);
                let mut busy = state.coordinator.current_busy().await;
                let mut force_emit = false;

                if busy != BusyState::None {
                    state.sidecar.touch_activity();
                } else if state.sidecar.is_online() {
                    let idle_minutes = state
                        .db
                        .get_settings()
                        .map(|s| s.auto_idle_unload_minutes)
                        .unwrap_or(10);
                    if idle_minutes > 0 {
                        let threshold = Duration::from_secs(u64::from(idle_minutes) * 60);
                        if state
                            .sidecar
                            .idle_duration()
                            .is_some_and(|elapsed| elapsed >= threshold)
                            && state.coordinator.ensure_idle_for_admin_action().await.is_ok()
                        {
                            let has_model =
                                state.db.get_active_model().ok().flatten().is_some();
                            state.sidecar.stop(has_model).await;
                            tracing::info!(
                                auto_idle_unload_minutes = idle_minutes,
                                "Auto-unloaded idle llama-server sidecar to free VRAM"
                            );
                            busy = state.coordinator.current_busy().await;
                            force_emit = true;
                            emit_vram_tick(&app);
                        }
                    }
                }

                if diag_open || force_emit || busy != BusyState::None || busy != last_busy {
                    let payload = build_current_telemetry(&state);
                    let _ = app.emit("engine:telemetry", payload);
                }
                last_busy = busy;
            }
        }
    });
}
