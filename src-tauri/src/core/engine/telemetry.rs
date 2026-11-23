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
}

pub fn build_current_telemetry(state: &AppState) -> EngineTelemetry {
    let (vram_used_mb, vram_total_mb) = dxgi::query_live_vram_mb().unwrap_or((0, 0));
    let db_wal_synced = state.db.check_wal_synced();
    let busy_by = state.coordinator.current_busy_nonblocking();
    state
        .sidecar
        .snapshot_telemetry(vram_used_mb, vram_total_mb, db_wal_synced, busy_by)
}

pub fn spawn_telemetry_loop(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut tick_count: u64 = 0;
        let mut last_busy = BusyState::None;
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;
            tick_count = tick_count.wrapping_add(1);

            if tick_count % 5 == 0 {
                if let Some((used, total)) = dxgi::query_live_vram_mb() {
                    let _ = app.emit(
                        "engine:vram_tick",
                        VramTickPayload {
                            vram_used_mb: used,
                            vram_total_mb: total,
                        },
                    );
                }
            }

            if tick_count % 2 == 0 {
                let diag_open = state.diagnostics_open.load(Ordering::Relaxed);
                let busy = state.coordinator.current_busy().await;
                if diag_open || busy != BusyState::None || busy != last_busy {
                    let payload = build_current_telemetry(&state);
                    let _ = app.emit("engine:telemetry", payload);
                }
                last_busy = busy;
            }
        }
    });
}
