use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

use super::batch_translator::{
    batch_size_for_ctx, prepare_cue_for_pipeline, translate_batch_with_healing,
};
use super::traits::{
    SubtitleBatchCompletedEvent, SubtitleBatchStartedEvent, SubtitleJobErrorEvent,
};
use crate::core::engine::concurrency::InferenceGuard;
use crate::core::engine::sidecar::SidecarConfig;
use crate::core::engine::telemetry::build_current_telemetry;
use crate::core::subtitle::exporter::{serialize_subtitle_document, write_atomic_export};
use crate::core::subtitle::tag_shield::shield_tags;
use crate::core::subtitle::types::CueMeta;
use crate::core::translator::prompts::Genre;
use crate::db::models::{GlossaryEntry, SubtitleCueDto, SubtitleJobSummary};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const STOP_NONE: u8 = 0;
const STOP_PAUSE: u8 = 1;
const STOP_CANCEL: u8 = 2;

#[derive(Clone)]
struct ActiveJobControl {
    cancel: CancellationToken,
    stop_mode: Arc<AtomicU8>,
    finished: Arc<Notify>,
}

/// Manages active subtitle job execution (`Pause`, `Resume`, `Cancel`), inline edits,
/// single-row re-translation, and 100%-completed export.
pub struct SubtitleOrchestrator {
    active_jobs: Arc<Mutex<HashMap<String, ActiveJobControl>>>,
}

impl Default for SubtitleOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl SubtitleOrchestrator {
    pub fn new() -> Self {
        Self {
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_or_resume_job(
        &self,
        state: &AppState,
        app: &AppHandle,
        job_id: &str,
    ) -> AppResult<SubtitleJobSummary> {
        let record = state.db.get_subtitle_job_record(job_id)?;
        if record.summary.status == "completed" {
            return Err(AppError::InvalidInput(
                "Subtitle job is already 100% completed.".to_string(),
            ));
        }

        let active_model = state
            .db
            .get_active_model()?
            .ok_or(AppError::NoActiveModel)?;

        let guard = state.coordinator.acquire_subtitle_job().await?;
        let sidecar_cfg = state.resolve_sidecar_config(&active_model).await?;

        state.db.reset_processing_cues_to_queued(job_id)?;
        state.db.update_subtitle_job_model_and_status(
            job_id,
            &active_model.id,
            &active_model.display_name,
            "running",
        )?;

        let summary = state.db.get_subtitle_job_summary(job_id)?;

        let cancel = CancellationToken::new();
        let stop_mode = Arc::new(AtomicU8::new(STOP_NONE));
        let finished = Arc::new(Notify::new());
        let control = ActiveJobControl {
            cancel: cancel.clone(),
            stop_mode: Arc::clone(&stop_mode),
            finished: Arc::clone(&finished),
        };

        {
            let mut active = self.active_jobs.lock().await;
            active.insert(job_id.to_string(), control);
        }

        let state_bg = state.clone();
        let app_bg = app.clone();
        let job_id_bg = job_id.to_string();
        let active_jobs_bg = Arc::clone(&self.active_jobs);

        tokio::spawn(async move {
            run_subtitle_job_worker(
                state_bg,
                app_bg,
                job_id_bg,
                sidecar_cfg,
                guard,
                cancel,
                stop_mode,
                finished,
                active_jobs_bg,
            )
            .await;
        });

        Ok(summary)
    }

    pub async fn pause_job(
        &self,
        state: &AppState,
        app: &AppHandle,
        job_id: &str,
    ) -> AppResult<SubtitleJobSummary> {
        self.stop_job_with_mode(state, app, job_id, STOP_PAUSE, "paused")
            .await
    }

    pub async fn cancel_job(
        &self,
        state: &AppState,
        app: &AppHandle,
        job_id: &str,
    ) -> AppResult<SubtitleJobSummary> {
        self.stop_job_with_mode(state, app, job_id, STOP_CANCEL, "cancelled")
            .await
    }

    async fn stop_job_with_mode(
        &self,
        state: &AppState,
        app: &AppHandle,
        job_id: &str,
        mode: u8,
        target_status: &str,
    ) -> AppResult<SubtitleJobSummary> {
        let control_opt = {
            let active = self.active_jobs.lock().await;
            active.get(job_id).cloned()
        };

        if let Some(ctrl) = control_opt {
            let wait_done = ctrl.finished.notified();
            ctrl.stop_mode.store(mode, Ordering::SeqCst);
            ctrl.cancel.cancel();
            let _ = tokio::time::timeout(Duration::from_secs(2), wait_done).await;
        } else {
            state.db.reset_processing_cues_to_queued(job_id)?;
            state.db.update_subtitle_job_status(job_id, target_status)?;
            let _ = app.emit("engine:telemetry", build_current_telemetry(state));
        }

        state.db.get_subtitle_job_summary(job_id)
    }

    pub async fn update_cue_translation(
        &self,
        state: &AppState,
        job_id: &str,
        cue_index: i64,
        edited_text: String,
    ) -> AppResult<SubtitleCueDto> {
        let summary = state.db.get_subtitle_job_summary(job_id)?;
        if summary.status == "running" {
            return Err(AppError::EngineBusySubtitleJob);
        }

        let cue = state.db.get_single_subtitle_cue(job_id, cue_index)?;
        if cue.cue_type != "dialogue" {
            return Err(AppError::InvalidInput(
                "Only dialogue cues can be manually edited.".to_string(),
            ));
        }

        let final_text = reattach_outer_tags_if_needed(
            &cue,
            edited_text.trim(),
            summary.tag_shield_enabled,
        );

        state.db.update_single_cue_and_recompute_job(
            job_id,
            cue_index,
            &final_text,
            "done",
            None,
            None,
        )
    }

    pub async fn retranslate_single_cue(
        &self,
        state: &AppState,
        app: &AppHandle,
        job_id: &str,
        cue_index: i64,
    ) -> AppResult<SubtitleCueDto> {
        let record = state.db.get_subtitle_job_record(job_id)?;
        if record.summary.status == "running" {
            return Err(AppError::EngineBusySubtitleJob);
        }

        let cue = state.db.get_single_subtitle_cue(job_id, cue_index)?;
        if cue.cue_type != "dialogue" {
            return Err(AppError::InvalidInput(
                "Only dialogue cues can be re-translated.".to_string(),
            ));
        }

        let _guard = state.coordinator.acquire_word_alternatives().await?;
        let active_model = state
            .db
            .get_active_model()?
            .ok_or(AppError::NoActiveModel)?;
        let sidecar_cfg = state.resolve_sidecar_config(&active_model).await?;

        let client = match state.sidecar.ensure_running(sidecar_cfg).await {
            Ok(c) => c,
            Err(err) => {
                if matches!(
                    err,
                    AppError::OutOfMemory
                        | AppError::VulkanInitFailed
                        | AppError::ByomFileMissing(_)
                        | AppError::SidecarCrashed(_)
                ) {
                    let _ = app.emit("engine:error", &err);
                }
                return Err(err);
            }
        };

        let glossary_entries = load_job_glossary(state, record.summary.glossary_id.as_deref())?;
        let prepared = vec![prepare_cue_for_pipeline(
            &cue,
            record.summary.tag_shield_enabled,
            &glossary_entries,
        )];
        let prev_context = state
            .db
            .get_previous_dialogue_context(job_id, cue_index, 3)?;
        let clean_context = clean_context_pairs(&prev_context, record.summary.tag_shield_enabled);
        let next_context = state
            .db
            .get_following_dialogue_context(job_id, cue_index, 2)?;
        let clean_next_context =
            clean_following_lines(&next_context, record.summary.tag_shield_enabled);

        let cancel = CancellationToken::new();
        let output = translate_batch_with_healing(
            &client,
            &prepared,
            &clean_context,
            &clean_next_context,
            Genre::from_str_lossy(&record.summary.genre),
            record.summary.tag_shield_enabled,
            &cancel,
        )
        .await?;

        state
            .sidecar
            .record_inference_metrics(output.avg_ttft_ms, output.avg_tok_per_sec);

        let outcome = output.outcomes.into_iter().next().ok_or_else(|| {
            AppError::InternalError("Single-cue re-translation produced no outcome.".to_string())
        })?;
        let glossary_json = serialize_glossary_matches(&prepared[0]);

        let updated = state.db.update_single_cue_and_recompute_job(
            job_id,
            cue_index,
            &outcome.translated_text,
            outcome.status,
            outcome.warning_msg.as_deref(),
            Some(&glossary_json),
        )?;

        drop(_guard);
        let _ = app.emit("engine:telemetry", build_current_telemetry(state));
        Ok(updated)
    }

    pub fn export_subtitle_file(
        &self,
        state: &AppState,
        job_id: &str,
        dest_path: &Path,
    ) -> AppResult<String> {
        let record = state.db.get_subtitle_job_record(job_id)?;
        if record.summary.status != "completed"
            || record.summary.completed_cues < record.summary.total_cues
        {
            return Err(AppError::InvalidInput(
                "Export is locked until 100% of subtitle cues are completed.".to_string(),
            ));
        }

        let cues = state.db.get_all_subtitle_cues(job_id)?;
        let content = serialize_subtitle_document(&record, &cues)?;
        write_atomic_export(dest_path, &content, false)?;

        let canonical = dunce::canonicalize(dest_path).unwrap_or_else(|_| dest_path.to_path_buf());
        Ok(canonical.to_string_lossy().into_owned())
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_subtitle_job_worker(
    state: AppState,
    app: AppHandle,
    job_id: String,
    sidecar_cfg: SidecarConfig,
    inference_guard: InferenceGuard,
    cancel: CancellationToken,
    stop_mode: Arc<AtomicU8>,
    finished: Arc<Notify>,
    active_jobs: Arc<Mutex<HashMap<String, ActiveJobControl>>>,
) {
    let _ = app.emit("engine:telemetry", build_current_telemetry(&state));

    let res = execute_job_batches(
        &state,
        &app,
        &job_id,
        &sidecar_cfg,
        &cancel,
        &stop_mode,
    )
    .await;

    if let Err(err) = res {
        if cancel.is_cancelled() {
            let final_status = if stop_mode.load(Ordering::SeqCst) == STOP_CANCEL {
                "cancelled"
            } else {
                "paused"
            };
            let _ = state.db.reset_processing_cues_to_queued(&job_id);
            let _ = state.db.update_subtitle_job_status(&job_id, final_status);
            if let Ok(summary) = state.db.get_subtitle_job_summary(&job_id) {
                let _ = app.emit(
                    "subtitle:batch_completed",
                    SubtitleBatchCompletedEvent {
                        job_id: job_id.clone(),
                        batch_number: 0,
                        cues: Vec::new(),
                        completed_cues: summary.completed_cues,
                        total_cues: summary.total_cues,
                        self_healing_errors: summary.self_healing_errors,
                        status: summary.status,
                        tok_per_sec: 0.0,
                    },
                );
            }
        } else {
            let _ = state.db.reset_processing_cues_to_queued(&job_id);
            let _ = state.db.update_subtitle_job_status(&job_id, "error");
            let code = error_code(&err).to_string();
            let message = err.to_string();
            if matches!(
                err,
                AppError::OutOfMemory
                    | AppError::VulkanInitFailed
                    | AppError::ByomFileMissing(_)
                    | AppError::SidecarCrashed(_)
            ) {
                let _ = app.emit("engine:error", &err);
            }
            let _ = app.emit(
                "subtitle:job_error",
                SubtitleJobErrorEvent {
                    job_id: job_id.clone(),
                    code,
                    message,
                },
            );
        }
    }

    {
        let mut active = active_jobs.lock().await;
        active.remove(&job_id);
    }
    drop(inference_guard);
    finished.notify_waiters();
    let _ = app.emit("engine:telemetry", build_current_telemetry(&state));
}

async fn execute_job_batches(
    state: &AppState,
    app: &AppHandle,
    job_id: &str,
    sidecar_cfg: &SidecarConfig,
    cancel: &CancellationToken,
    stop_mode: &AtomicU8,
) -> AppResult<()> {
    let record = state.db.get_subtitle_job_record(job_id)?;
    let glossary_entries = load_job_glossary(state, record.summary.glossary_id.as_deref())?;
    let all_cues = state.db.get_all_subtitle_cues(job_id)?;

    let mut next_batch_number = all_cues
        .iter()
        .filter_map(|c| c.batch_number)
        .max()
        .unwrap_or(0)
        + 1;

    let queued: Vec<SubtitleCueDto> = all_cues
        .into_iter()
        .filter(|c| c.cue_type == "dialogue" && c.status == "queued")
        .collect();

    if queued.is_empty() {
        let (summary, updated_cues) =
            state
                .db
                .commit_batch_cues(job_id, next_batch_number, &[], Some("completed"))?;
        let _ = app.emit(
            "subtitle:batch_completed",
            SubtitleBatchCompletedEvent {
                job_id: job_id.to_string(),
                batch_number: next_batch_number,
                cues: updated_cues,
                completed_cues: summary.completed_cues,
                total_cues: summary.total_cues,
                self_healing_errors: summary.self_healing_errors,
                status: summary.status,
                tok_per_sec: 0.0,
            },
        );
        return Ok(());
    }

    let genre = Genre::from_str_lossy(&record.summary.genre);
    let tag_shield_enabled = record.summary.tag_shield_enabled;
    let batch_size = batch_size_for_ctx(sidecar_cfg.ctx_size);
    let chunks: Vec<&[SubtitleCueDto]> = queued.chunks(batch_size).collect();
    let total_chunks = chunks.len();

    for (chunk_idx, chunk) in chunks.into_iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(AppError::InternalError(
                "Subtitle job paused or cancelled".to_string(),
            ));
        }

        let batch_number = next_batch_number;
        next_batch_number += 1;
        let cue_indices: Vec<i64> = chunk.iter().map(|c| c.cue_index).collect();

        state
            .db
            .mark_cues_processing(job_id, &cue_indices, batch_number)?;
        let _ = app.emit(
            "subtitle:batch_started",
            SubtitleBatchStartedEvent {
                job_id: job_id.to_string(),
                batch_number,
                cue_indices: cue_indices.clone(),
            },
        );

        let client = tokio::select! {
            _ = cancel.cancelled() => {
                return Err(AppError::InternalError("Subtitle job paused or cancelled".to_string()));
            }
            res = state.sidecar.ensure_running(sidecar_cfg.clone()) => res?
        };

        let _ = app.emit("engine:telemetry", build_current_telemetry(state));

        let prepared: Vec<_> = chunk
            .iter()
            .map(|c| prepare_cue_for_pipeline(c, tag_shield_enabled, &glossary_entries))
            .collect();

        let prev_context = state
            .db
            .get_previous_dialogue_context(job_id, chunk[0].cue_index, 3)?;
        let clean_context = clean_context_pairs(&prev_context, tag_shield_enabled);
        let last_cue_idx = chunk.last().map(|c| c.cue_index).unwrap_or(chunk[0].cue_index);
        let next_context = state
            .db
            .get_following_dialogue_context(job_id, last_cue_idx, 2)?;
        let clean_next_context = clean_following_lines(&next_context, tag_shield_enabled);

        let batch_out = translate_batch_with_healing(
            &client,
            &prepared,
            &clean_context,
            &clean_next_context,
            genre,
            tag_shield_enabled,
            cancel,
        )
        .await?;

        if cancel.is_cancelled() {
            return Err(AppError::InternalError(
                "Subtitle job paused or cancelled".to_string(),
            ));
        }

        state
            .sidecar
            .record_inference_metrics(batch_out.avg_ttft_ms, batch_out.avg_tok_per_sec);

        let mut updates = Vec::with_capacity(batch_out.outcomes.len());
        for (prep, outcome) in prepared.iter().zip(batch_out.outcomes.into_iter()) {
            let glossary_json = serialize_glossary_matches(prep);
            updates.push((
                outcome.cue_index,
                outcome.translated_text,
                outcome.status.to_string(),
                outcome.warning_msg,
                Some(glossary_json),
            ));
        }

        let is_last = chunk_idx + 1 == total_chunks;
        let next_status = if is_last { Some("completed") } else { None };
        let (summary, updated_cues) =
            state
                .db
                .commit_batch_cues(job_id, batch_number, &updates, next_status)?;

        let _ = app.emit(
            "subtitle:batch_completed",
            SubtitleBatchCompletedEvent {
                job_id: job_id.to_string(),
                batch_number,
                cues: updated_cues,
                completed_cues: summary.completed_cues,
                total_cues: summary.total_cues,
                self_healing_errors: summary.self_healing_errors,
                status: summary.status,
                tok_per_sec: batch_out.avg_tok_per_sec,
            },
        );
        let _ = app.emit("engine:telemetry", build_current_telemetry(state));
    }

    if stop_mode.load(Ordering::SeqCst) == STOP_CANCEL {
        state.db.update_subtitle_job_status(job_id, "cancelled")?;
    }

    Ok(())
}

fn load_job_glossary(
    state: &AppState,
    glossary_id: Option<&str>,
) -> AppResult<Vec<GlossaryEntry>> {
    match glossary_id {
        Some(id) if !id.trim().is_empty() => state.db.list_glossary_entries(id),
        _ => Ok(Vec::new()),
    }
}

fn clean_context_pairs(raw_pairs: &[(String, String)], tag_shield_enabled: bool) -> Vec<(String, String)> {
    raw_pairs
        .iter()
        .map(|(src, dst)| {
            if tag_shield_enabled {
                (
                    shield_tags(src).clean_text,
                    shield_tags(dst).clean_text,
                )
            } else {
                (src.clone(), dst.clone())
            }
        })
        .collect()
}

fn clean_following_lines(raw_lines: &[String], tag_shield_enabled: bool) -> Vec<String> {
    raw_lines
        .iter()
        .map(|src| {
            if tag_shield_enabled {
                shield_tags(src).clean_text
            } else {
                src.clone()
            }
        })
        .collect()
}

fn serialize_glossary_matches(prep: &super::traits::PreparedCue) -> String {
    serde_json::to_string(&prep.glossary_matches).unwrap_or_else(|_| "[]".to_string())
}

fn reattach_outer_tags_if_needed(
    cue: &SubtitleCueDto,
    edited_text: &str,
    tag_shield_enabled: bool,
) -> String {
    if !tag_shield_enabled {
        return edited_text.to_string();
    }

    let meta: CueMeta = cue
        .meta_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default();

    let (leading, trailing) = if meta.leading_tags.is_some() || meta.trailing_tags.is_some() {
        (
            meta.leading_tags.unwrap_or_default(),
            meta.trailing_tags.unwrap_or_default(),
        )
    } else {
        let computed = shield_tags(&cue.original_text);
        (computed.leading_tags, computed.trailing_tags)
    };

    let mut out = String::with_capacity(leading.len() + edited_text.len() + trailing.len());
    if !leading.is_empty() && !edited_text.starts_with(&leading) {
        out.push_str(&leading);
    }
    out.push_str(edited_text);
    if !trailing.is_empty() && !out.ends_with(&trailing) {
        out.push_str(&trailing);
    }
    out
}

fn error_code(err: &AppError) -> &'static str {
    match err {
        AppError::OutOfMemory => "OUT_OF_MEMORY",
        AppError::VulkanInitFailed => "VULKAN_INIT_FAILED",
        AppError::ByomFileMissing(_) => "BYOM_FILE_MISSING",
        AppError::EngineBusySubtitleJob => "ENGINE_BUSY_SUBTITLE_JOB",
        AppError::SidecarCrashed(_) => "SIDECAR_CRASHED",
        AppError::NoActiveModel => "NO_ACTIVE_MODEL",
        AppError::InvalidInput(_) => "INVALID_INPUT",
        AppError::InvalidPath(_) => "INVALID_PATH",
        AppError::NotFound(_) => "NOT_FOUND",
        AppError::DbError(_) => "DB_ERROR",
        AppError::IoError(_) => "IO_ERROR",
        AppError::HttpError(_) => "HTTP_ERROR",
        AppError::DiskSpaceInsufficient(_) => "DISK_SPACE_INSUFFICIENT",
        AppError::Sha256Mismatch(_) => "SHA256_MISMATCH",
        AppError::InternalError(_) => "INTERNAL_ERROR",
    }
}
