use crate::core::translator::prompts::Tone;
use crate::core::translator::text_translator::{
    run_text_translation, TranslationResult, TranslationStreamEvent,
};
use crate::core::translator::word_alternatives::{
    run_word_alternatives, WordAlternativesResponse,
};
use crate::db::models::TextHistoryItem;
use crate::db::repositories::now_unix_ms;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn translate_text(
    app: AppHandle,
    state: State<'_, AppState>,
    source_text: String,
    tone: String,
    on_stream: Channel<TranslationStreamEvent>,
) -> AppResult<TranslationResult> {
    run_text_translation(&state, &app, source_text, tone, move |evt| {
        let _ = on_stream.send(evt);
    })
    .await
}

#[tauri::command]
pub async fn cancel_text_translation(state: State<'_, AppState>) -> AppResult<()> {
    state.coordinator.cancel_text_stream().await;
    Ok(())
}

#[tauri::command]
pub async fn get_word_alternatives(
    app: AppHandle,
    state: State<'_, AppState>,
    word: String,
    sentence_context: String,
    source_text: String,
) -> AppResult<WordAlternativesResponse> {
    run_word_alternatives(&state, &app, word, sentence_context, source_text).await
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn save_text_history(
    state: State<'_, AppState>,
    source_text: String,
    translated_text: String,
    tone: String,
    model_id: String,
    model_display_name: String,
    ttft_ms: Option<i64>,
    tok_per_sec: Option<f64>,
) -> AppResult<TextHistoryItem> {
    let clean_source = source_text.trim().to_string();
    let clean_target = translated_text.trim().to_string();
    if clean_source.is_empty() || clean_target.is_empty() {
        return Err(AppError::InvalidInput(
            "Source and translated text cannot be empty when saving to history".to_string(),
        ));
    }

    let normalized_tone = Tone::from_str_lossy(&tone).as_str().to_string();
    if let Some(latest) = state.db.get_latest_text_history()? {
        if latest.source_text == clean_source
            && latest.translated_text == clean_target
            && latest.tone == normalized_tone
        {
            return Ok(latest);
        }
    }

    let item = TextHistoryItem {
        id: uuid::Uuid::new_v4().to_string(),
        source_text: clean_source,
        translated_text: clean_target,
        tone: normalized_tone,
        model_id,
        model_display_name,
        ttft_ms,
        tok_per_sec,
        created_at: now_unix_ms(),
    };
    state.db.insert_text_history(&item)?;
    Ok(item)
}

#[tauri::command]
pub async fn list_text_history(
    state: State<'_, AppState>,
    query: Option<String>,
    limit: Option<u32>,
) -> AppResult<Vec<TextHistoryItem>> {
    state
        .db
        .list_text_history(query.as_deref(), limit.unwrap_or(100))
}

#[tauri::command]
pub async fn delete_text_history(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.db.delete_text_history(&id)
}
