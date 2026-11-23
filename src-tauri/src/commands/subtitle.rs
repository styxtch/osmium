use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::core::subtitle::glossary_matcher::match_glossary_terms;
use crate::core::subtitle::parse_subtitle_file;
use crate::core::subtitle::tag_shield::shield_tags;
use crate::core::subtitle::types::CueType;
use crate::core::translator::prompts::Genre;
use crate::db::models::{
    NewSubtitleCue, SubtitleCueDto, SubtitleJobRecord, SubtitleJobSummary,
};
use crate::db::repositories::now_unix_ms;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[tauri::command]
pub async fn parse_and_create_subtitle_job(
    state: State<'_, AppState>,
    file_path: String,
    genre: Option<String>,
    glossary_id: Option<String>,
    tag_shield_enabled: Option<bool>,
) -> AppResult<SubtitleJobSummary> {
    let raw_path = PathBuf::from(&file_path);
    let canonical = dunce::canonicalize(&raw_path).unwrap_or(raw_path.clone());

    let parsed_path = canonical.clone();
    let (doc, file_size_bytes) =
        tokio::task::spawn_blocking(move || parse_subtitle_file(&parsed_path))
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))??;

    let clean_glossary_id = glossary_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let glossary_entries = match clean_glossary_id.as_deref() {
        Some(gid) => state.db.list_glossary_entries(gid)?,
        None => Vec::new(),
    };

    let normalized_genre = Genre::from_str_lossy(genre.as_deref().unwrap_or("general"))
        .as_str()
        .to_string();
    let shield_active = tag_shield_enabled.unwrap_or(true);

    let active_model = state.db.get_active_model()?;
    let (model_id, model_display_name) = match active_model {
        Some(m) => (m.id, m.display_name),
        None => (String::new(), "No Active Model".to_string()),
    };

    let file_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("subtitle")
        .to_string();

    let mut new_cues = Vec::with_capacity(doc.cues.len());
    let mut translatable_total = 0i64;

    for cue in doc.cues {
        let mut meta = cue.meta.clone();
        let shield = shield_tags(&cue.original_text);

        if !shield.leading_tags.is_empty() {
            meta.leading_tags = Some(shield.leading_tags.clone());
        }
        if !shield.trailing_tags.is_empty() {
            meta.trailing_tags = Some(shield.trailing_tags.clone());
        }
        if !shield.display_tags.is_empty() {
            meta.display_tags = Some(shield.display_tags.clone());
        }

        let effective_type = if shield.is_drawing {
            CueType::Drawing
        } else {
            cue.cue_type
        };

        let (initial_status, glossary_matches_json) = if effective_type == CueType::Dialogue {
            translatable_total += 1;
            let matches = match_glossary_terms(&shield.clean_text, &glossary_entries);
            let gm_json = serde_json::to_string(&matches).ok();
            ("queued".to_string(), gm_json)
        } else {
            ("skipped".to_string(), None)
        };

        let meta_json = serde_json::to_string(&meta).ok();

        new_cues.push(NewSubtitleCue {
            cue_index: cue.cue_index,
            cue_type: effective_type.as_str().to_string(),
            start_time: cue.start_time,
            end_time: cue.end_time,
            meta_json,
            glossary_matches_json,
            original_text: cue.original_text,
            translated_text: None,
            status: initial_status,
        });
    }

    let now = now_unix_ms();
    let job_record = SubtitleJobRecord {
        summary: SubtitleJobSummary {
            id: Uuid::new_v4().to_string(),
            file_name,
            file_path: canonical.to_string_lossy().into_owned(),
            file_size_bytes,
            format: doc.format.as_str().to_string(),
            genre: normalized_genre,
            glossary_id: clean_glossary_id,
            tag_shield_enabled: shield_active,
            model_id,
            model_display_name,
            status: "idle".to_string(),
            total_cues: translatable_total,
            completed_cues: 0,
            self_healing_errors: 0,
            created_at: now,
            updated_at: now,
        },
        raw_header: doc.raw_header,
        raw_footer: doc.raw_footer,
        ass_format_cols: doc
            .ass_format_cols
            .and_then(|cols| serde_json::to_string(&cols).ok()),
    };

    state.db.create_subtitle_job(&job_record, &new_cues)?;
    Ok(job_record.summary)
}

#[tauri::command]
pub async fn list_subtitle_jobs(state: State<'_, AppState>) -> AppResult<Vec<SubtitleJobSummary>> {
    state.db.list_subtitle_jobs()
}

#[tauri::command]
pub async fn get_subtitle_job(
    state: State<'_, AppState>,
    job_id: String,
) -> AppResult<SubtitleJobSummary> {
    state.db.get_subtitle_job_summary(&job_id)
}

#[tauri::command]
pub async fn delete_subtitle_job(state: State<'_, AppState>, job_id: String) -> AppResult<()> {
    let summary = state.db.get_subtitle_job_summary(&job_id)?;
    if summary.status == "running" {
        return Err(AppError::EngineBusySubtitleJob);
    }
    state.db.delete_subtitle_job(&job_id)
}

#[tauri::command]
pub async fn update_subtitle_job_options(
    state: State<'_, AppState>,
    job_id: String,
    genre: String,
    glossary_id: Option<String>,
    tag_shield_enabled: bool,
) -> AppResult<SubtitleJobSummary> {
    let current = state.db.get_subtitle_job_summary(&job_id)?;
    if current.status == "running" {
        return Err(AppError::EngineBusySubtitleJob);
    }

    let normalized_genre = Genre::from_str_lossy(&genre).as_str().to_string();
    let clean_glossary_id = glossary_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    state.db.update_subtitle_job_options(
        &job_id,
        &normalized_genre,
        clean_glossary_id.as_deref(),
        tag_shield_enabled,
    )
}

#[tauri::command]
pub async fn get_subtitle_cues(
    state: State<'_, AppState>,
    job_id: String,
    offset: u32,
    limit: u32,
    search_query: Option<String>,
    status_filter: Option<String>,
) -> AppResult<Vec<SubtitleCueDto>> {
    state.db.get_subtitle_cues_filtered(
        &job_id,
        offset,
        limit,
        search_query.as_deref(),
        status_filter.as_deref(),
    )
}

#[tauri::command]
pub async fn start_subtitle_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> AppResult<SubtitleJobSummary> {
    state
        .subtitle_orchestrator
        .start_or_resume_job(&state, &app, &job_id)
        .await
}

#[tauri::command]
pub async fn pause_subtitle_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> AppResult<SubtitleJobSummary> {
    state
        .subtitle_orchestrator
        .pause_job(&state, &app, &job_id)
        .await
}

#[tauri::command]
pub async fn resume_subtitle_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> AppResult<SubtitleJobSummary> {
    state
        .subtitle_orchestrator
        .start_or_resume_job(&state, &app, &job_id)
        .await
}

#[tauri::command]
pub async fn cancel_subtitle_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> AppResult<SubtitleJobSummary> {
    state
        .subtitle_orchestrator
        .cancel_job(&state, &app, &job_id)
        .await
}

#[tauri::command]
pub async fn update_cue_translation(
    state: State<'_, AppState>,
    job_id: String,
    cue_index: i64,
    translated_text: String,
) -> AppResult<SubtitleCueDto> {
    state
        .subtitle_orchestrator
        .update_cue_translation(&state, &job_id, cue_index, translated_text)
        .await
}

#[tauri::command]
pub async fn retranslate_single_cue(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    cue_index: i64,
) -> AppResult<SubtitleCueDto> {
    state
        .subtitle_orchestrator
        .retranslate_single_cue(&state, &app, &job_id, cue_index)
        .await
}

#[tauri::command]
pub async fn export_subtitle_file(
    state: State<'_, AppState>,
    job_id: String,
    dest_path: String,
) -> AppResult<String> {
    state
        .subtitle_orchestrator
        .export_subtitle_file(&state, &job_id, Path::new(&dest_path))
}
