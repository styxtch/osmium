use crate::core::engine::llama_client::ChatCompletionRequest;
use crate::core::engine::telemetry::build_current_telemetry;
use crate::core::translator::chunker::{chunk_threshold_for_ctx, split_into_chunks};
use crate::core::translator::languages::{normalize_source_lang, normalize_target_lang};
use crate::core::translator::prompts::{build_text_translation_messages, Tone};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationStreamEvent {
    pub event_type: String, // "status" | "chunk" | "done" | "cancelled"
    pub delta: Option<String>,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub ttft_ms: Option<u64>,
    pub tok_per_sec: Option<f64>,
    pub total_tokens: Option<u32>,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub tone: String,
    pub model_id: String,
    pub model_display_name: String,
    pub ttft_ms: u64,
    pub tok_per_sec: f64,
    pub total_tokens: u32,
    pub chunk_count: usize,
    pub cancelled: bool,
}

pub async fn run_text_translation<F>(
    state: &AppState,
    app: &AppHandle,
    source_text: String,
    source_lang_raw: Option<String>,
    target_lang_raw: Option<String>,
    tone_raw: String,
    on_event: F,
) -> AppResult<TranslationResult>
where
    F: Fn(TranslationStreamEvent) + Send + Sync,
{
    let tone = Tone::from_str_lossy(&tone_raw);
    let source_lang = normalize_source_lang(source_lang_raw.as_deref());
    let target_lang = normalize_target_lang(target_lang_raw.as_deref());
    let (_guard, cancel) = state.coordinator.acquire_text_stream().await?;

    let active_model = state
        .db
        .get_active_model()?
        .ok_or(AppError::NoActiveModel)?;

    let sidecar_cfg = state.resolve_sidecar_config(&active_model).await?;
    let chunk_threshold = chunk_threshold_for_ctx(sidecar_cfg.ctx_size);
    let chunks = split_into_chunks(&source_text, chunk_threshold)?;
    let total_chunks = chunks.len();

    on_event(TranslationStreamEvent {
        event_type: "status".to_string(),
        delta: None,
        chunk_index: 0,
        total_chunks,
        ttft_ms: None,
        tok_per_sec: None,
        total_tokens: None,
        status_message: Some("Preparing local AI engine...".to_string()),
    });
    let _ = app.emit("engine:telemetry", build_current_telemetry(state));

    let client = match state.sidecar.ensure_running(sidecar_cfg).await {
        Ok(c) => c,
        Err(err) => {
            if matches!(
                err,
                AppError::OutOfMemory | AppError::VulkanInitFailed | AppError::ByomFileMissing(_)
            ) {
                let _ = app.emit("engine:error", &err);
            }
            let _ = app.emit("engine:telemetry", build_current_telemetry(state));
            return Err(err);
        }
    };

    let _ = app.emit("engine:telemetry", build_current_telemetry(state));
    on_event(TranslationStreamEvent {
        event_type: "status".to_string(),
        delta: None,
        chunk_index: 0,
        total_chunks,
        ttft_ms: None,
        tok_per_sec: None,
        total_tokens: None,
        status_message: Some("Streaming translation...".to_string()),
    });

    let overall_start = Instant::now();
    let mut full_output = String::new();
    let mut first_ttft_ms: Option<u64> = None;
    let mut total_tokens: u32 = 0;
    let mut was_cancelled = false;

    for (idx, chunk_text) in chunks.iter().enumerate() {
        if cancel.is_cancelled() {
            was_cancelled = true;
            break;
        }

        if idx > 0 {
            full_output.push_str("\n\n");
            on_event(TranslationStreamEvent {
                event_type: "chunk".to_string(),
                delta: Some("\n\n".to_string()),
                chunk_index: idx,
                total_chunks,
                ttft_ms: first_ttft_ms,
                tok_per_sec: None,
                total_tokens: None,
                status_message: None,
            });
        }

        let temperature = match tone {
            Tone::Formal => 0.1,
            Tone::Natural => 0.2,
            Tone::Casual => 0.35,
        };

        let req = ChatCompletionRequest {
            messages: build_text_translation_messages(chunk_text, source_lang, target_lang, tone),
            temperature,
            max_tokens: None,
            response_format: None,
            disable_thinking: true,
        };

        let metrics = client
            .chat_completion_stream(req, cancel.clone(), |delta| {
                on_event(TranslationStreamEvent {
                    event_type: "chunk".to_string(),
                    delta: Some(delta.to_string()),
                    chunk_index: idx,
                    total_chunks,
                    ttft_ms: first_ttft_ms,
                    tok_per_sec: None,
                    total_tokens: None,
                    status_message: None,
                });
            })
            .await?;

        if first_ttft_ms.is_none() && metrics.ttft_ms > 0 {
            first_ttft_ms = Some(metrics.ttft_ms);
        }
        total_tokens = total_tokens.saturating_add(metrics.total_tokens);
        full_output.push_str(&metrics.full_text);

        if metrics.cancelled {
            was_cancelled = true;
            break;
        }
    }

    let elapsed_secs = overall_start.elapsed().as_secs_f64().max(0.001);
    let agg_tok_per_sec = if total_tokens > 0 {
        ((f64::from(total_tokens) / elapsed_secs) * 10.0).round() / 10.0
    } else {
        0.0
    };
    let ttft_ms = first_ttft_ms.unwrap_or(0);

    if !was_cancelled {
        state
            .sidecar
            .record_inference_metrics(ttft_ms, agg_tok_per_sec);
    }

    on_event(TranslationStreamEvent {
        event_type: if was_cancelled { "cancelled" } else { "done" }.to_string(),
        delta: None,
        chunk_index: total_chunks,
        total_chunks,
        ttft_ms: Some(ttft_ms),
        tok_per_sec: Some(agg_tok_per_sec),
        total_tokens: Some(total_tokens),
        status_message: None,
    });

    drop(_guard);
    let _ = app.emit("engine:telemetry", build_current_telemetry(state));

    Ok(TranslationResult {
        translated_text: full_output.trim().to_string(),
        source_lang: source_lang.to_string(),
        target_lang: target_lang.to_string(),
        tone: tone.as_str().to_string(),
        model_id: active_model.id,
        model_display_name: active_model.display_name,
        ttft_ms,
        tok_per_sec: agg_tok_per_sec,
        total_tokens,
        chunk_count: total_chunks,
        cancelled: was_cancelled,
    })
}
