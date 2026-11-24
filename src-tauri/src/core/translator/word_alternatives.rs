use crate::core::engine::llama_client::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, LlamaClient,
};
use crate::core::engine::sanitizer::strip_think_tags;
use crate::core::engine::telemetry::build_current_telemetry;
use crate::core::translator::languages::normalize_target_lang;
use crate::core::translator::prompts::{
    build_word_alternatives_messages, word_alternatives_json_schema,
};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WordAlternativeItem {
    pub word: String,
    pub nuance: String,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WordAlternativesResponse {
    pub context_summary: String,
    pub alternatives: Vec<WordAlternativeItem>,
}

pub async fn run_word_alternatives(
    state: &AppState,
    app: &AppHandle,
    word: String,
    sentence_context: String,
    source_text: String,
    target_lang_raw: Option<String>,
) -> AppResult<WordAlternativesResponse> {
    if word.trim().is_empty() || sentence_context.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Word and sentence context cannot be empty".to_string(),
        ));
    }

    let target_lang = normalize_target_lang(target_lang_raw.as_deref());

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
                AppError::OutOfMemory | AppError::VulkanInitFailed | AppError::ByomFileMissing(_)
            ) {
                let _ = app.emit("engine:error", &err);
            }
            return Err(err);
        }
    };

    let messages = build_word_alternatives_messages(
        &word,
        &sentence_context,
        &source_text,
        target_lang,
    );
    let resp = request_word_alternatives_completion(&client, &messages, 0.35, 512).await?;
    state
        .sidecar
        .record_inference_metrics(resp.ttft_ms, resp.tok_per_sec);

    let parsed = match parse_word_alternatives_json(&resp.content) {
        Ok(ok) => ok,
        Err(_) => {
            // Single resilient retry with a larger token budget and lower temperature in case output was truncated.
            let retry_resp =
                request_word_alternatives_completion(&client, &messages, 0.2, 768).await?;
            state
                .sidecar
                .record_inference_metrics(retry_resp.ttft_ms, retry_resp.tok_per_sec);
            parse_word_alternatives_json(&retry_resp.content)?
        }
    };

    drop(_guard);
    let _ = app.emit("engine:telemetry", build_current_telemetry(state));
    Ok(parsed)
}

async fn request_word_alternatives_completion(
    client: &LlamaClient,
    messages: &[ChatMessage],
    temperature: f32,
    max_tokens: u32,
) -> AppResult<ChatCompletionResponse> {
    let schema_req = ChatCompletionRequest {
        messages: messages.to_vec(),
        temperature,
        max_tokens: Some(max_tokens),
        response_format: Some(word_alternatives_json_schema()),
        disable_thinking: true,
    };

    match client.chat_completion_json(schema_req).await {
        Ok(r) => Ok(r),
        Err(AppError::HttpError(_)) => {
            // Fallback for custom BYOM models/templates that reject JSON Schema grammar
            let fallback_req = ChatCompletionRequest {
                messages: messages.to_vec(),
                temperature,
                max_tokens: Some(max_tokens),
                response_format: None,
                disable_thinking: true,
            };
            client.chat_completion_json(fallback_req).await
        }
        Err(e) => Err(e),
    }
}

pub fn parse_word_alternatives_json(raw: &str) -> AppResult<WordAlternativesResponse> {
    let stripped = strip_think_tags(raw);
    let json_slice = extract_json_object(&stripped).ok_or_else(|| {
        AppError::InternalError(
            "Model did not return word alternatives in valid JSON format"
                .to_string(),
        )
    })?;

    let mut parsed: WordAlternativesResponse = serde_json::from_str(json_slice).map_err(|e| {
        AppError::InternalError(format!("Failed to parse word alternatives JSON: {e}"))
    })?;

    parsed.context_summary = parsed.context_summary.trim().to_string();
    parsed.alternatives.retain(|a| !a.word.trim().is_empty());
    for item in &mut parsed.alternatives {
        item.word = item.word.trim().to_string();
        item.nuance = item.nuance.trim().to_string();
    }
    parsed.alternatives.truncate(5);

    if parsed.alternatives.is_empty() {
        return Err(AppError::InternalError(
            "Model returned no word alternatives".to_string(),
        ));
    }

    if !parsed.alternatives.iter().any(|a| a.recommended) {
        parsed.alternatives[0].recommended = true;
    }

    Ok(parsed)
}

fn extract_json_object(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end > start {
        Some(&trimmed[start..=end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_strict_and_fenced_word_alternatives_json() {
        let strict = r#"{
            "context_summary": "keputusan strategis",
            "alternatives": [
                { "word": "tegas", "nuance": "Lebih profesional.", "recommended": true },
                { "word": "berani", "nuance": "Lebih ekspresif.", "recommended": false }
            ]
        }"#;
        let res = parse_word_alternatives_json(strict).unwrap();
        assert_eq!(res.context_summary, "keputusan strategis");
        assert_eq!(res.alternatives.len(), 2);
        assert!(res.alternatives[0].recommended);

        let fenced = "Berikut opsinya:\n```json\n{\"context_summary\":\"aksi cepat\",\"alternatives\":[{\"word\":\"kilat\",\"nuance\":\"Sangat cepat\",\"recommended\":false}]}\n```";
        let res2 = parse_word_alternatives_json(fenced).unwrap();
        assert_eq!(res2.alternatives[0].word, "kilat");
        assert!(res2.alternatives[0].recommended, "Fallback sets first item recommended");

        assert!(parse_word_alternatives_json("not json").is_err());

        // Truncated multi-line JSON missing closing brackets must error cleanly
        let truncated = "{\n  \"context_summary\": \"pertemuan kembali\",\n  \"alternatives\": [\n    {\"word\": \"menemui\"";
        assert!(parse_word_alternatives_json(truncated).is_err());

        // Compact single-line JSON with <think> wrapper parses cleanly
        let with_think = "<think>reasoning</think>{\"context_summary\":\"bertemu lagi\",\"alternatives\":[{\"word\":\"menemui\",\"nuance\":\"Lebih formal dan personal.\",\"recommended\":true},{\"word\":\"berjumpa\",\"nuance\":\"Lebih puitis dan sastra.\",\"recommended\":false}]}";
        let res3 = parse_word_alternatives_json(with_think).unwrap();
        assert_eq!(res3.alternatives.len(), 2);
        assert_eq!(res3.alternatives[0].word, "menemui");
    }
}
