use crate::core::engine::concurrency::BusyState;
use crate::core::engine::job_object::Win32JobObject;
use crate::core::engine::llama_client::LlamaClient;
use crate::core::engine::telemetry::EngineTelemetry;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex as AsyncMutex;

const STDERR_RING_CAPACITY: usize = 50;
const HEALTH_POLL_INTERVAL_MS: u64 = 250;
const HEALTH_POLL_MAX_ATTEMPTS: usize = 240; // 60s

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SidecarStatus {
    Unconfigured,
    Standby,
    Loading,
    Online,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarConfig {
    pub model_id: String,
    pub model_display_name: String,
    pub model_path: PathBuf,
    pub backend: String,
    pub vulkan_device_id: Option<String>,
    pub ctx_size: u32,
    pub gpu_layers: u32,
    pub bin_dir: PathBuf,
}

struct RunningSidecar {
    child: Child,
    _job: Win32JobObject,
    config: SidecarConfig,
    port: u16,
    api_key: String,
    _stderr_lines: Arc<StdMutex<VecDeque<String>>>,
}

struct TelemetrySnapshotState {
    status: SidecarStatus,
    pid: Option<u32>,
    port: Option<u16>,
    active_model_id: Option<String>,
    active_model_name: Option<String>,
    backend: String,
    ctx_size: u32,
    gpu_layers: u32,
    last_ttft_ms: Option<u64>,
    last_tok_per_sec: Option<f64>,
}

pub struct SidecarManager {
    running: AsyncMutex<Option<RunningSidecar>>,
    snapshot: StdMutex<TelemetrySnapshotState>,
}

impl Default for SidecarManager {
    fn default() -> Self {
        Self::new(false)
    }
}

impl SidecarManager {
    pub fn new(has_active_model: bool) -> Self {
        Self {
            running: AsyncMutex::new(None),
            snapshot: StdMutex::new(TelemetrySnapshotState {
                status: if has_active_model {
                    SidecarStatus::Standby
                } else {
                    SidecarStatus::Unconfigured
                },
                pid: None,
                port: None,
                active_model_id: None,
                active_model_name: None,
                backend: "vulkan".to_string(),
                ctx_size: 4096,
                gpu_layers: 99,
                last_ttft_ms: None,
                last_tok_per_sec: None,
            }),
        }
    }

    pub fn set_standby_model(
        &self,
        model_id: Option<String>,
        model_name: Option<String>,
        backend: String,
        ctx_size: u32,
        gpu_layers: u32,
    ) {
        if let Ok(mut snap) = self.snapshot.lock() {
            snap.active_model_id = model_id.clone();
            snap.active_model_name = model_name;
            snap.backend = backend;
            snap.ctx_size = ctx_size;
            snap.gpu_layers = gpu_layers;
            if snap.pid.is_none() {
                snap.status = if model_id.is_some() {
                    SidecarStatus::Standby
                } else {
                    SidecarStatus::Unconfigured
                };
            }
        }
    }

    pub fn record_inference_metrics(&self, ttft_ms: u64, tok_per_sec: f64) {
        if let Ok(mut snap) = self.snapshot.lock() {
            snap.last_ttft_ms = Some(ttft_ms);
            snap.last_tok_per_sec = Some((tok_per_sec * 10.0).round() / 10.0);
        }
    }

    pub async fn ensure_running(&self, config: SidecarConfig) -> AppResult<LlamaClient> {
        let mut guard = self.running.lock().await;

        if let Some(existing) = guard.as_mut() {
            let alive = matches!(existing.child.try_wait(), Ok(None));
            if alive && existing.config == config {
                let client = LlamaClient::new(existing.port, existing.api_key.clone())?;
                if matches!(client.health_check().await, Ok(true)) {
                    return Ok(client);
                }
            }
            Self::terminate_running(&mut guard).await;
        }

        if !config.model_path.exists() {
            self.update_status_error(&config);
            return Err(AppError::ByomFileMissing(
                config.model_path.to_string_lossy().to_string(),
            ));
        }

        let exe_path = config.bin_dir.join("llama-server.exe");
        if !exe_path.exists() {
            self.update_status_error(&config);
            return Err(AppError::SidecarCrashed(format!(
                "llama-server.exe binary was not found at {}",
                exe_path.display()
            )));
        }

        let port = allocate_ephemeral_port()?;
        let api_key = generate_session_token();

        {
            if let Ok(mut snap) = self.snapshot.lock() {
                snap.status = SidecarStatus::Loading;
                snap.pid = None;
                snap.port = Some(port);
                snap.active_model_id = Some(config.model_id.clone());
                snap.active_model_name = Some(config.model_display_name.clone());
                snap.backend = config.backend.clone();
                snap.ctx_size = config.ctx_size;
                snap.gpu_layers = config.gpu_layers;
            }
        }

        let args = build_launch_args(&config, port);
        let mut cmd = tokio::process::Command::new(&exe_path);
        cmd.args(&args)
            .env("LLAMA_API_KEY", &api_key)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(|e| {
            self.update_status_error(&config);
            AppError::SidecarCrashed(format!("Failed to start llama-server.exe: {e}"))
        })?;

        let pid = child.id();
        let job = Win32JobObject::new()?;
        #[cfg(windows)]
        if let Some(raw_handle) = child.raw_handle() {
            if let Err(err) = job.assign_process_handle(raw_handle) {
                let _ = child.start_kill();
                self.update_status_error(&config);
                return Err(err);
            }
        }

        let stderr_lines = Arc::new(StdMutex::new(VecDeque::with_capacity(STDERR_RING_CAPACITY)));
        if let Some(stderr) = child.stderr.take() {
            let ring = Arc::clone(&stderr_lines);
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Ok(mut q) = ring.lock() {
                        if q.len() >= STDERR_RING_CAPACITY {
                            q.pop_front();
                        }
                        q.push_back(line);
                    }
                }
            });
        }

        let client = LlamaClient::new(port, api_key.clone())?;

        for _ in 0..HEALTH_POLL_MAX_ATTEMPTS {
            if let Ok(Some(exit_status)) = child.try_wait() {
                let captured: Vec<String> = stderr_lines
                    .lock()
                    .map(|q| q.iter().cloned().collect())
                    .unwrap_or_default();
                drop(job);
                self.update_status_error(&config);
                if let Some(classified) = classify_stderr_error(&captured) {
                    return Err(classified);
                }
                return Err(AppError::SidecarCrashed(format!(
                    "llama-server exited during startup (exit: {:?})",
                    exit_status.code()
                )));
            }

            if matches!(client.health_check().await, Ok(true)) {
                if let Ok(mut snap) = self.snapshot.lock() {
                    snap.status = SidecarStatus::Online;
                    snap.pid = pid;
                    snap.port = Some(port);
                }
                *guard = Some(RunningSidecar {
                    child,
                    _job: job,
                    config,
                    port,
                    api_key,
                    _stderr_lines: stderr_lines,
                });
                tracing::info!(port, ?pid, "llama-server sidecar is online");
                return Ok(client);
            }

            tokio::time::sleep(Duration::from_millis(HEALTH_POLL_INTERVAL_MS)).await;
        }

        let _ = child.start_kill();
        drop(job);
        self.update_status_error(&config);
        Err(AppError::SidecarCrashed(
            "Timed out waiting for llama-server /health readiness (60 seconds)".to_string(),
        ))
    }

    pub async fn stop(&self, has_active_model: bool) {
        let mut guard = self.running.lock().await;
        Self::terminate_running(&mut guard).await;
        if let Ok(mut snap) = self.snapshot.lock() {
            snap.pid = None;
            snap.port = None;
            snap.status = if has_active_model {
                SidecarStatus::Standby
            } else {
                SidecarStatus::Unconfigured
            };
        }
    }

    pub fn snapshot_telemetry(
        &self,
        vram_used_mb: u64,
        vram_total_mb: u64,
        db_wal_synced: bool,
        busy_by: BusyState,
    ) -> EngineTelemetry {
        let snap = self.snapshot.lock().ok();
        match snap {
            Some(s) => EngineTelemetry {
                status: s.status,
                pid: s.pid,
                port: s.port,
                active_model_id: s.active_model_id.clone(),
                active_model_name: s.active_model_name.clone(),
                backend: s.backend.clone(),
                ctx_size: s.ctx_size,
                gpu_layers: s.gpu_layers,
                vram_used_mb,
                vram_total_mb,
                last_ttft_ms: s.last_ttft_ms,
                last_tok_per_sec: s.last_tok_per_sec,
                db_wal_synced,
                busy_by,
            },
            None => EngineTelemetry {
                status: SidecarStatus::Error,
                pid: None,
                port: None,
                active_model_id: None,
                active_model_name: None,
                backend: "cpu".to_string(),
                ctx_size: 2048,
                gpu_layers: 0,
                vram_used_mb,
                vram_total_mb,
                last_ttft_ms: None,
                last_tok_per_sec: None,
                db_wal_synced,
                busy_by,
            },
        }
    }

    async fn terminate_running(guard: &mut Option<RunningSidecar>) {
        if let Some(mut proc) = guard.take() {
            let _ = proc.child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(3), proc.child.wait()).await;
            tracing::info!(port = proc.port, "llama-server sidecar stopped");
        }
    }

    fn update_status_error(&self, config: &SidecarConfig) {
        if let Ok(mut snap) = self.snapshot.lock() {
            snap.status = SidecarStatus::Error;
            snap.pid = None;
            snap.port = None;
            snap.active_model_id = Some(config.model_id.clone());
            snap.active_model_name = Some(config.model_display_name.clone());
        }
    }
}

pub fn build_launch_args(config: &SidecarConfig, port: u16) -> Vec<String> {
    let mut args = vec![
        "--model".to_string(),
        config.model_path.to_string_lossy().to_string(),
        "--host".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        port.to_string(),
        "--ctx-size".to_string(),
        config.ctx_size.to_string(),
        "--parallel".to_string(),
        "1".to_string(),
        "--no-webui".to_string(),
        "--reasoning".to_string(),
        "off".to_string(),
    ];

    if config.backend == "vulkan" && config.gpu_layers > 0 {
        if let Some(dev) = &config.vulkan_device_id {
            args.push("--device".to_string());
            args.push(dev.clone());
        }
        args.extend([
            "--flash-attn".to_string(),
            "on".to_string(),
            "--cache-type-k".to_string(),
            "q8_0".to_string(),
            "--cache-type-v".to_string(),
            "q8_0".to_string(),
            "--n-gpu-layers".to_string(),
            config.gpu_layers.to_string(),
        ]);
        if config.gpu_layers >= 99 {
            args.extend(["--load-mode".to_string(), "none".to_string()]);
        }
    } else {
        args.extend([
            "--device".to_string(),
            "none".to_string(),
            "--n-gpu-layers".to_string(),
            "0".to_string(),
        ]);
    }

    args
}

pub fn classify_stderr_error(lines: &[String]) -> Option<AppError> {
    let combined = lines.join("\n").to_lowercase();
    if combined.contains("out of memory")
        || combined.contains("vk_error_out_of_device_memory")
        || combined.contains("failed to allocate")
        || combined.contains("insufficient memory")
    {
        return Some(AppError::OutOfMemory);
    }

    if combined.contains("vk_error_device_lost")
        || combined.contains("vk_error_initialization_failed")
        || ((combined.contains("ggml_vulkan") || combined.contains("vulkan"))
            && (combined.contains("failed")
                || combined.contains("error")
                || combined.contains("unsupported")))
    {
        return Some(AppError::VulkanInitFailed);
    }

    None
}

fn allocate_ephemeral_port() -> AppResult<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn generate_session_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_args_never_contain_api_key_and_respect_backend() {
        let vulkan_cfg = SidecarConfig {
            model_id: "qwen3-8b-q4km".to_string(),
            model_display_name: "Qwen 3 8B".to_string(),
            model_path: PathBuf::from("C:\\Models\\Qwen3-8B-Q4_K_M.gguf"),
            backend: "vulkan".to_string(),
            vulkan_device_id: Some("Vulkan0".to_string()),
            ctx_size: 4096,
            gpu_layers: 99,
            bin_dir: PathBuf::from("C:\\Bin"),
        };

        let vk_args = build_launch_args(&vulkan_cfg, 54321);
        assert!(!vk_args.iter().any(|a| a.contains("--api-key")));
        assert!(vk_args.windows(2).any(|w| w == ["--device", "Vulkan0"]));
        assert!(vk_args.windows(2).any(|w| w == ["--flash-attn", "on"]));
        assert!(vk_args.windows(2).any(|w| w == ["--cache-type-k", "q8_0"]));
        assert!(vk_args.windows(2).any(|w| w == ["--parallel", "1"]));
        assert!(vk_args.contains(&"--no-webui".to_string()));
        assert!(vk_args.windows(2).any(|w| w == ["--load-mode", "none"]));

        let cpu_cfg = SidecarConfig {
            backend: "cpu".to_string(),
            gpu_layers: 0,
            ..vulkan_cfg
        };
        let cpu_args = build_launch_args(&cpu_cfg, 54322);
        assert!(cpu_args.windows(2).any(|w| w == ["--device", "none"]));
        assert!(cpu_args.windows(2).any(|w| w == ["--n-gpu-layers", "0"]));
        assert!(!cpu_args.contains(&"--load-mode".to_string()));
    }

    #[test]
    fn stderr_classifier_detects_oom_and_vulkan_failures() {
        let oom = vec![
            "ggml_vulkan: allocating buffer...".to_string(),
            "Error: VK_ERROR_OUT_OF_DEVICE_MEMORY failed to allocate".to_string(),
        ];
        assert!(matches!(classify_stderr_error(&oom), Some(AppError::OutOfMemory)));

        let vk_lost = vec!["ggml_vulkan: initialization failed: VK_ERROR_INITIALIZATION_FAILED".to_string()];
        assert!(matches!(
            classify_stderr_error(&vk_lost),
            Some(AppError::VulkanInitFailed)
        ));

        let benign = vec!["srv log_server_r: starting server".to_string()];
        assert!(classify_stderr_error(&benign).is_none());
    }
}
