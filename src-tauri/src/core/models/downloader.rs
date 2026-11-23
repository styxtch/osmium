use crate::core::hardware::disk;
use crate::core::models::catalog::CatalogModel;
use crate::core::models::gguf_validator::validate_gguf_file;
use crate::db::models::{DownloadTask, InstalledModel};
use crate::db::repositories::now_unix_ms;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

const DISK_MARGIN_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB safety margin
const STOP_NONE: u8 = 0;
const STOP_PAUSE: u8 = 1;
const STOP_CANCEL: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgressEvent {
    pub model_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percent: f64,
    pub speed_mb_per_sec: f64,
    pub eta_seconds: Option<u64>,
    pub status: String,
    pub error_msg: Option<String>,
}

struct ActiveDownloadHandle {
    cancel: CancellationToken,
    stop_reason: Arc<AtomicU8>,
}

#[derive(Default)]
pub struct DownloadCoordinator {
    active: Arc<Mutex<HashMap<String, ActiveDownloadHandle>>>,
}

impl DownloadCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn pause(&self, model_id: &str) -> bool {
        let guard = self.active.lock().await;
        if let Some(handle) = guard.get(model_id) {
            handle.stop_reason.store(STOP_PAUSE, Ordering::SeqCst);
            handle.cancel.cancel();
            true
        } else {
            false
        }
    }

    pub async fn cancel(&self, model_id: &str, db: &Database) -> AppResult<()> {
        let active_found = {
            let guard = self.active.lock().await;
            if let Some(handle) = guard.get(model_id) {
                handle.stop_reason.store(STOP_CANCEL, Ordering::SeqCst);
                handle.cancel.cancel();
                true
            } else {
                false
            }
        };

        if !active_found {
            if let Some(task) = db.get_download_task(model_id)? {
                let _ = fs::remove_file(&task.part_path);
                db.delete_download_task(model_id)?;
            }
        }
        Ok(())
    }

    pub async fn start_or_resume<F>(
        &self,
        db: &Database,
        catalog_model: &CatalogModel,
        target_dir: &Path,
        on_progress: F,
    ) -> AppResult<DownloadTask>
    where
        F: Fn(DownloadProgressEvent) + Send + Sync + 'static,
    {
        let model_id = catalog_model.id.clone();

        let (cancel, stop_reason) = {
            let mut guard = self.active.lock().await;
            if guard.contains_key(&model_id) {
                return Err(AppError::InvalidInput(
                    "A download for this model is already in progress".to_string(),
                ));
            }
            let cancel = CancellationToken::new();
            let stop_reason = Arc::new(AtomicU8::new(STOP_NONE));
            guard.insert(
                model_id.clone(),
                ActiveDownloadHandle {
                    cancel: cancel.clone(),
                    stop_reason: Arc::clone(&stop_reason),
                },
            );
            (cancel, stop_reason)
        };

        let result = run_download_flow(
            db,
            catalog_model,
            target_dir,
            cancel,
            stop_reason,
            on_progress,
        )
        .await;

        let mut guard = self.active.lock().await;
        guard.remove(&model_id);
        result
    }
}

async fn run_download_flow<F>(
    db: &Database,
    catalog_model: &CatalogModel,
    target_dir: &Path,
    cancel: CancellationToken,
    stop_reason: Arc<AtomicU8>,
    on_progress: F,
) -> AppResult<DownloadTask>
where
    F: Fn(DownloadProgressEvent) + Send + Sync + 'static,
{
    let validated_dir = disk::validate_user_dir(target_dir)?;
    let target_path = validated_dir.join(&catalog_model.file_name);
    let part_path = validated_dir.join(format!("{}.part", catalog_model.file_name));

    let existing_task = db.get_download_task(&catalog_model.id)?;
    let mut current_offset = if part_path.exists() {
        fs::metadata(&part_path)?.len()
    } else {
        0
    };
    let mut stored_etag = existing_task.and_then(|t| t.etag);

    let remaining = catalog_model.file_size_bytes.saturating_sub(current_offset);
    let disk_info = disk::check_disk_space(&validated_dir)?;
    if !disk_info.writable {
        return Err(AppError::InvalidPath(format!(
            "Destination folder is not writable: {}",
            validated_dir.display()
        )));
    }
    if disk_info.free_bytes < remaining.saturating_add(DISK_MARGIN_BYTES) {
        return Err(AppError::DiskSpaceInsufficient(format!(
            "At least {:.2} GB of free space is required ({:.2} GB available)",
            (remaining.saturating_add(DISK_MARGIN_BYTES)) as f64 / (1024.0 * 1024.0 * 1024.0),
            disk_info.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
        )));
    }

    let parsed_url = reqwest::Url::parse(&catalog_model.download_url)
        .map_err(|e| AppError::InvalidInput(format!("Invalid download URL: {e}")))?;
    if !is_allowed_download_url(&parsed_url) {
        return Err(AppError::InvalidInput(
            "Download host is outside the official HuggingFace allowlist".to_string(),
        ));
    }

    let client = build_allowlisted_http_client()?;
    let mut req = client.get(parsed_url);
    if current_offset > 0 && current_offset < catalog_model.file_size_bytes {
        req = req.header(reqwest::header::RANGE, format!("bytes={current_offset}-"));
        if let Some(ref etag) = stored_etag {
            req = req.header(reqwest::header::IF_RANGE, etag.as_str());
        }
    } else if current_offset >= catalog_model.file_size_bytes && catalog_model.file_size_bytes > 0 {
        current_offset = 0;
    }

    let mut task = DownloadTask {
        model_id: catalog_model.id.clone(),
        download_url: catalog_model.download_url.clone(),
        target_path: target_path.to_string_lossy().to_string(),
        part_path: part_path.to_string_lossy().to_string(),
        expected_sha256: catalog_model.sha256.clone(),
        etag: stored_etag.clone(),
        total_bytes: catalog_model.file_size_bytes as i64,
        bytes_downloaded: current_offset as i64,
        status: "downloading".to_string(),
        error_msg: None,
        updated_at: now_unix_ms(),
    };
    db.upsert_download_task(&task)?;

    let mut resp = match req.send().await {
        Ok(r) => r,
        Err(err) => {
            task.status = "paused".to_string();
            task.error_msg = Some("Download paused due to network interruption. Progress is saved safely and can be resumed.".to_string());
            task.updated_at = now_unix_ms();
            let _ = db.upsert_download_task(&task);
            return Err(AppError::HttpError(err.to_string()));
        }
    };

    let status_code = resp.status();
    if !status_code.is_success() {
        task.status = "failed".to_string();
        task.error_msg = Some(format!("Server returned HTTP {status_code}"));
        task.updated_at = now_unix_ms();
        let _ = db.upsert_download_task(&task);
        return Err(AppError::HttpError(format!(
            "Failed to download model: HTTP {status_code}"
        )));
    }

    if let Some(etag_val) = resp
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
    {
        stored_etag = Some(etag_val.to_string());
        task.etag = stored_etag.clone();
    }

    let mut file: File = if status_code == reqwest::StatusCode::PARTIAL_CONTENT && current_offset > 0 {
        OpenOptions::new().create(true).append(true).open(&part_path)?
    } else {
        current_offset = 0;
        task.bytes_downloaded = 0;
        File::create(&part_path)?
    };

    let total_bytes = catalog_model.file_size_bytes.max(1);
    let session_start_bytes = current_offset;
    let session_start = Instant::now();
    let mut last_emit = Instant::now();
    let mut last_db_sync = Instant::now();

    on_progress(make_progress_event(
        &catalog_model.id,
        current_offset,
        total_bytes,
        0.0,
        None,
        "downloading",
        None,
    ));

    loop {
        let next_chunk = tokio::select! {
            _ = cancel.cancelled() => {
                let reason = stop_reason.load(Ordering::SeqCst);
                let _ = file.flush();
                drop(file);

                if reason == STOP_CANCEL {
                    let _ = fs::remove_file(&part_path);
                    let _ = db.delete_download_task(&catalog_model.id);
                    on_progress(make_progress_event(
                        &catalog_model.id,
                        0,
                        total_bytes,
                        0.0,
                        None,
                        "cancelled",
                        None,
                    ));
                    task.status = "cancelled".to_string();
                    task.bytes_downloaded = 0;
                    return Ok(task);
                } else {
                    task.bytes_downloaded = current_offset as i64;
                    task.status = "paused".to_string();
                    task.error_msg = None;
                    task.updated_at = now_unix_ms();
                    db.upsert_download_task(&task)?;
                    on_progress(make_progress_event(
                        &catalog_model.id,
                        current_offset,
                        total_bytes,
                        0.0,
                        None,
                        "paused",
                        None,
                    ));
                    return Ok(task);
                }
            }
            chunk = resp.chunk() => chunk,
        };

        let bytes = match next_chunk {
            Ok(Some(b)) => b,
            Ok(None) => break,
            Err(err) => {
                let _ = file.flush();
                task.bytes_downloaded = current_offset as i64;
                task.status = "paused".to_string();
                task.error_msg = Some(
                    "Download paused due to network interruption. Progress is saved safely and can be resumed.".to_string(),
                );
                task.updated_at = now_unix_ms();
                let _ = db.upsert_download_task(&task);
                on_progress(make_progress_event(
                    &catalog_model.id,
                    current_offset,
                    total_bytes,
                    0.0,
                    None,
                    "paused",
                    task.error_msg.clone(),
                ));
                return Err(AppError::HttpError(err.to_string()));
            }
        };

        file.write_all(&bytes)?;
        current_offset += bytes.len() as u64;

        let now = Instant::now();
        if now.duration_since(last_db_sync) >= Duration::from_millis(750) {
            task.bytes_downloaded = current_offset as i64;
            task.updated_at = now_unix_ms();
            let _ = db.upsert_download_task(&task);
            last_db_sync = now;
        }

        if now.duration_since(last_emit) >= Duration::from_millis(200) {
            let elapsed = session_start.elapsed().as_secs_f64().max(0.05);
            let session_bytes = current_offset.saturating_sub(session_start_bytes) as f64;
            let bytes_per_sec = session_bytes / elapsed;
            let speed_mb = bytes_per_sec / (1024.0 * 1024.0);
            let eta_seconds = if bytes_per_sec > 1024.0 {
                Some((total_bytes.saturating_sub(current_offset) as f64 / bytes_per_sec).ceil() as u64)
            } else {
                None
            };
            on_progress(make_progress_event(
                &catalog_model.id,
                current_offset,
                total_bytes,
                speed_mb,
                eta_seconds,
                "downloading",
                None,
            ));
            last_emit = now;
        }
    }

    file.flush()?;
    drop(file);

    task.bytes_downloaded = current_offset as i64;
    task.status = "verifying".to_string();
    task.updated_at = now_unix_ms();
    db.upsert_download_task(&task)?;

    on_progress(make_progress_event(
        &catalog_model.id,
        current_offset,
        total_bytes,
        0.0,
        Some(0),
        "verifying",
        None,
    ));

    let part_clone = part_path.clone();
    let expected_sha = catalog_model.sha256.clone();
    let verified = tokio::task::spawn_blocking(move || verify_file_sha256(&part_clone, &expected_sha))
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))??;

    if !verified {
        let _ = fs::remove_file(&part_path);
        task.status = "failed".to_string();
        task.bytes_downloaded = 0;
        task.error_msg = Some(
            "Downloaded model file is corrupted or does not match the official checksum. Please re-download.".to_string(),
        );
        task.updated_at = now_unix_ms();
        let _ = db.upsert_download_task(&task);
        on_progress(make_progress_event(
            &catalog_model.id,
            0,
            total_bytes,
            0.0,
            None,
            "failed",
            task.error_msg.clone(),
        ));
        return Err(AppError::Sha256Mismatch(catalog_model.file_name.clone()));
    }

    if target_path.exists() {
        let _ = fs::remove_file(&target_path);
    }
    fs::rename(&part_path, &target_path)?;

    let gguf_meta = validate_gguf_file(&target_path).ok();
    let should_activate = db.get_active_model()?.is_none();
    let installed = InstalledModel {
        id: catalog_model.id.clone(),
        display_name: catalog_model.display_name.clone(),
        tier: catalog_model.tier.clone(),
        file_path: target_path.to_string_lossy().to_string(),
        file_size_bytes: current_offset as i64,
        sha256: Some(catalog_model.sha256.clone()),
        quantization: Some(catalog_model.quantization.clone()),
        is_byom: false,
        block_count: gguf_meta
            .and_then(|m| m.block_count)
            .map(i64::from)
            .or(Some(i64::from(catalog_model.block_count))),
        is_active: should_activate,
        created_at: now_unix_ms(),
    };
    db.upsert_installed_model(&installed)?;
    db.delete_download_task(&catalog_model.id)?;

    task.status = "completed".to_string();
    on_progress(make_progress_event(
        &catalog_model.id,
        current_offset,
        total_bytes,
        0.0,
        Some(0),
        "completed",
        None,
    ));

    tracing::info!(model_id = %catalog_model.id, "Official model downloaded and verified via SHA-256");
    Ok(task)
}

fn make_progress_event(
    model_id: &str,
    bytes_downloaded: u64,
    total_bytes: u64,
    speed_mb_per_sec: f64,
    eta_seconds: Option<u64>,
    status: &str,
    error_msg: Option<String>,
) -> DownloadProgressEvent {
    let percent = if total_bytes > 0 {
        ((bytes_downloaded as f64 / total_bytes as f64) * 100.0).min(100.0)
    } else {
        0.0
    };
    DownloadProgressEvent {
        model_id: model_id.to_string(),
        bytes_downloaded,
        total_bytes,
        percent: (percent * 10.0).round() / 10.0,
        speed_mb_per_sec: (speed_mb_per_sec * 10.0).round() / 10.0,
        eta_seconds,
        status: status.to_string(),
        error_msg,
    }
}

pub fn is_allowed_download_url(url: &reqwest::Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let h = host.to_ascii_lowercase();
    h == "huggingface.co"
        || h.ends_with(".huggingface.co")
        || h == "hf.co"
        || h.ends_with(".hf.co")
}

fn build_allowlisted_http_client() -> AppResult<reqwest::Client> {
    let policy = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() > 8 {
            return attempt.error("Too many download redirects");
        }
        if is_allowed_download_url(attempt.url()) {
            attempt.follow()
        } else {
            attempt.error("Redirect points outside the official HuggingFace allowlist")
        }
    });

    reqwest::Client::builder()
        .https_only(true)
        .redirect(policy)
        .connect_timeout(Duration::from_secs(15))
        .user_agent("Osmium-Translator/0.1.0")
        .build()
        .map_err(AppError::from)
}

pub fn verify_file_sha256(path: &Path, expected_hex: &str) -> AppResult<bool> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let actual_hex = hex::encode(hasher.finalize());
    Ok(actual_hex.eq_ignore_ascii_case(expected_hex.trim()))
}

pub fn path_is_valid_dir(path: &str) -> PathBuf {
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_permits_only_https_huggingface_hosts() {
        let valid_urls = [
            "https://huggingface.co/Qwen/Qwen3-8B-GGUF/resolve/main/Qwen3-8B-Q4_K_M.gguf",
            "https://cdn-lfs-us-1.hf.co/repos/abc/def",
            "https://cas-bridge.xethub.hf.co/xet-bridge-us/123",
            "https://cdn-lfs.huggingface.co/file",
        ];
        for raw in valid_urls {
            let url = reqwest::Url::parse(raw).unwrap();
            assert!(is_allowed_download_url(&url), "Should allow {raw}");
        }

        let blocked_urls = [
            "http://huggingface.co/Qwen/Qwen3-8B-GGUF",
            "https://evil-huggingface.co/model.gguf",
            "https://example.com/model.gguf",
            "https://github.com/ggml-org/llama.cpp",
        ];
        for raw in blocked_urls {
            let url = reqwest::Url::parse(raw).unwrap();
            assert!(!is_allowed_download_url(&url), "Should reject {raw}");
        }
    }

    #[test]
    fn streaming_sha256_verification_matches_and_detects_mismatch() {
        let temp = std::env::temp_dir().join(format!("osmium_sha_{}.bin", uuid::Uuid::new_v4()));
        fs::write(&temp, b"osmium-local-ai-translator").unwrap();

        let expected = hex::encode(Sha256::digest(b"osmium-local-ai-translator"));
        assert!(verify_file_sha256(&temp, &expected).unwrap());
        assert!(!verify_file_sha256(&temp, &"0".repeat(64)).unwrap());

        let _ = fs::remove_file(&temp);
    }
}
