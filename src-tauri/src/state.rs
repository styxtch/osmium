use crate::core::engine::concurrency::InferenceCoordinator;
use crate::core::engine::sidecar::{SidecarConfig, SidecarManager};
use crate::core::hardware::types::HardwareProfile;
use crate::core::hardware::{
    estimate_partial_offload_layers, recommended_ctx_size, scan_hardware,
};
use crate::core::models::downloader::DownloadCoordinator;
use crate::core::pipeline::orchestrator::SubtitleOrchestrator;
use crate::db::models::InstalledModel;
use crate::db::Database;
use crate::error::AppResult;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_appender::non_blocking::WorkerGuard;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub sidecar: Arc<SidecarManager>,
    pub coordinator: Arc<InferenceCoordinator>,
    pub downloads: Arc<DownloadCoordinator>,
    pub subtitle_orchestrator: Arc<SubtitleOrchestrator>,
    pub bin_dir: PathBuf,
    pub hardware_cache: Arc<RwLock<Option<HardwareProfile>>>,
    pub diagnostics_open: Arc<AtomicBool>,
    _log_guard: Arc<Option<WorkerGuard>>,
}

impl AppState {
    pub fn new(
        db: Database,
        sidecar: Arc<SidecarManager>,
        coordinator: Arc<InferenceCoordinator>,
        bin_dir: PathBuf,
        log_guard: Option<WorkerGuard>,
    ) -> Self {
        Self {
            db,
            sidecar,
            coordinator,
            downloads: Arc::new(DownloadCoordinator::new()),
            subtitle_orchestrator: Arc::new(SubtitleOrchestrator::new()),
            bin_dir,
            hardware_cache: Arc::new(RwLock::new(None)),
            diagnostics_open: Arc::new(AtomicBool::new(false)),
            _log_guard: Arc::new(log_guard),
        }
    }

    pub async fn get_or_scan_hardware(&self) -> AppResult<HardwareProfile> {
        if let Some(cached) = self.hardware_cache.read().await.clone() {
            return Ok(cached);
        }
        let bin_dir = self.bin_dir.clone();
        let profile = tokio::task::spawn_blocking(move || scan_hardware(Some(&bin_dir)))
            .await
            .map_err(|e| crate::error::AppError::InternalError(e.to_string()))??;
        *self.hardware_cache.write().await = Some(profile.clone());
        Ok(profile)
    }

    pub async fn resolve_sidecar_config(&self, model: &InstalledModel) -> AppResult<SidecarConfig> {
        let settings = self.db.get_settings()?;
        let hw = self.get_or_scan_hardware().await?;

        let use_vulkan = hw.vulkan_supported && settings.backend_override != "cpu";
        let backend = if use_vulkan {
            "vulkan".to_string()
        } else {
            "cpu".to_string()
        };

        let ctx_size = match settings.ctx_size_override.as_str() {
            "2048" => 2048,
            "4096" => 4096,
            "8192" => 8192,
            _ => recommended_ctx_size(hw.dedicated_vram_mb, use_vulkan),
        };

        let gpu_layers = if !use_vulkan {
            0
        } else if let Ok(manual_layers) = settings.gpu_layers_override.parse::<u32>() {
            manual_layers
        } else {
            estimate_partial_offload_layers(
                hw.dedicated_vram_mb,
                model.file_size_bytes.max(0) as u64,
                model.block_count.map(|b| b.max(0) as u32),
            )
        };

        Ok(SidecarConfig {
            model_id: model.id.clone(),
            model_display_name: model.display_name.clone(),
            model_path: PathBuf::from(&model.file_path),
            backend,
            vulkan_device_id: hw.vulkan_device_id,
            ctx_size,
            gpu_layers,
            bin_dir: self.bin_dir.clone(),
        })
    }
}
