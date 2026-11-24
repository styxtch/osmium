use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::traits::{
    BatchCompletionClient, BatchTranslationOutput, CueOutcome, PreparedCue,
};
use crate::core::engine::llama_client::{ChatCompletionRequest, ChatMessage};
use crate::core::engine::sanitizer::strip_think_tags;
use crate::core::subtitle::glossary_matcher::match_glossary_terms;
use crate::core::subtitle::tag_shield::{restore_tags, shield_tags};
use crate::core::subtitle::types::CueType;
use crate::core::translator::languages::{language_info, language_name};
use crate::core::translator::prompts::Genre;
use crate::db::models::{GlossaryEntry, SubtitleCueDto};
use crate::error::{AppError, AppResult};

#[derive(Debug, Serialize, Deserialize)]
struct BatchPromptItem {
    id: i64,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BatchJsonPayload {
    items: Vec<BatchPromptItem>,
}

#[derive(Default)]
struct MetricsAccumulator {
    total_tok_per_sec: f64,
    total_ttft_ms: u64,
    calls: u32,
}

impl MetricsAccumulator {
    fn record(&mut self, ttft_ms: u64, tok_per_sec: f64) {
        self.total_ttft_ms = self.total_ttft_ms.saturating_add(ttft_ms);
        self.total_tok_per_sec += tok_per_sec;
        self.calls += 1;
    }

    fn averages(&self) -> (u64, f64) {
        if self.calls == 0 {
            (0, 0.0)
        } else {
            (
                self.total_ttft_ms / u64::from(self.calls),
                self.total_tok_per_sec / f64::from(self.calls),
            )
        }
    }
}

/// Computes safe batch size matching active `--ctx-size` window.
pub fn batch_size_for_ctx(ctx_size: u32) -> usize {
    if ctx_size >= 8192 {
        18
    } else if ctx_size >= 4096 {
        12
    } else {
        6
    }
}

/// JSON schema passed to `llama-server` constrained grammar decoding.
pub fn subtitle_batch_json_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "subtitle_batch_translation",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": { "type": "integer" },
                                "text": { "type": "string" }
                            },
                            "required": ["id", "text"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["items"],
                "additionalProperties": false
            }
        }
    })
}

/// Prepares a stored cue with Tag Shield placeholders and Glossary matches prior to LLM prompting.
pub fn prepare_cue_for_pipeline(
    cue: &SubtitleCueDto,
    tag_shield_enabled: bool,
    glossary_entries: &[GlossaryEntry],
) -> PreparedCue {
    let parsed_type = CueType::from_str_lossy(&cue.cue_type);
    let computed_shield = shield_tags(&cue.original_text);

    let cue_type = if computed_shield.is_drawing {
        CueType::Drawing
    } else {
        parsed_type
    };

    let (source_for_prompt, active_shield) = if tag_shield_enabled {
        (computed_shield.clean_text.clone(), Some(computed_shield))
    } else {
        (cue.original_text.clone(), None)
    };

    let glossary_source = active_shield
        .as_ref()
        .map(|s| s.clean_text.as_str())
        .unwrap_or(&cue.original_text);
    let glossary_matches = match_glossary_terms(glossary_source, glossary_entries);

    PreparedCue {
        cue_index: cue.cue_index,
        cue_type,
        original_text: cue.original_text.clone(),
        source_for_prompt,
        shield: active_shield,
        glossary_matches,
    }
}

/// Translates a batch of dialogue cues using **Bounded Self-Healing**:
/// 1. Batch call at `temperature = 0.2` with JSON Schema (or plain JSON fallback for BYOM).
/// 2. Verify JSON structure, exact ID matching, and Tag Shield placeholder integrity.
/// 3. On mismatch (`len > 1`), retry batch ONCE at `temperature = 0.1` with strict instructions.
/// 4. If retry still fails (`len > 1`), **Recursive Binary Split Fallback** into halves down to `len == 1`.
/// 5. A single cue (`len == 1`) gets max 1 retry (`temperature = 0.05`); if still invalid, returns
///    `status = "warning"` keeping `original_text` intact (guaranteed zero infinite loops).
#[allow(clippy::too_many_arguments)]
pub async fn translate_batch_with_healing<C: BatchCompletionClient>(
    client: &C,
    batch: &[PreparedCue],
    prev_context: &[(String, String)],
    next_context: &[String],
    source_lang: &str,
    target_lang: &str,
    genre: Genre,
    tag_shield_enabled: bool,
    cancel: &CancellationToken,
) -> AppResult<BatchTranslationOutput> {
    let mut metrics = MetricsAccumulator::default();
    let mut outcomes = Vec::with_capacity(batch.len());

    // Handle trivially empty/non-dialogue cues immediately without wasting an LLM call.
    let mut translatable: Vec<PreparedCue> = Vec::with_capacity(batch.len());
    let mut non_dialogue_outcomes: HashMap<i64, CueOutcome> = HashMap::new();

    for cue in batch {
        if cue.cue_type != CueType::Dialogue || cue.source_for_prompt.trim().is_empty() {
            non_dialogue_outcomes.insert(
                cue.cue_index,
                CueOutcome {
                    cue_index: cue.cue_index,
                    translated_text: cue.original_text.clone(),
                    status: if cue.cue_type == CueType::Dialogue {
                        "done"
                    } else {
                        "skipped"
                    },
                    warning_msg: None,
                    self_healed: false,
                },
            );
        } else {
            translatable.push(cue.clone());
        }
    }

    let mut translated_map: HashMap<i64, CueOutcome> = HashMap::new();
    if !translatable.is_empty() {
        translate_slice_bounded(
            client,
            &translatable,
            prev_context,
            next_context,
            source_lang,
            target_lang,
            genre,
            tag_shield_enabled,
            true,
            cancel,
            &mut metrics,
            &mut translated_map,
        )
        .await?;
    }

    for cue in batch {
        if let Some(outcome) = non_dialogue_outcomes.remove(&cue.cue_index) {
            outcomes.push(outcome);
        } else if let Some(outcome) = translated_map.remove(&cue.cue_index) {
            outcomes.push(outcome);
        }
    }

    let (avg_ttft_ms, avg_tok_per_sec) = metrics.averages();
    Ok(BatchTranslationOutput {
        outcomes,
        avg_tok_per_sec,
        avg_ttft_ms,
    })
}

#[allow(clippy::too_many_arguments)]
fn translate_slice_bounded<'a, C: BatchCompletionClient>(
    client: &'a C,
    slice: &'a [PreparedCue],
    prev_context: &'a [(String, String)],
    next_context: &'a [String],
    source_lang: &'a str,
    target_lang: &'a str,
    genre: Genre,
    tag_shield_enabled: bool,
    allow_initial_batch_retry: bool,
    cancel: &'a CancellationToken,
    metrics: &'a mut MetricsAccumulator,
    out_map: &'a mut HashMap<i64, CueOutcome>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = AppResult<()>> + Send + 'a>> {
    Box::pin(async move {
        if cancel.is_cancelled() {
            return Err(AppError::InternalError("Subtitle job paused or cancelled".to_string()));
        }

        if slice.is_empty() {
            return Ok(());
        }

        if slice.len() == 1 {
            let cue = &slice[0];
            // Attempt 1 for single cue
            let mut last_err: Option<String> = None;
            for (attempt, temp) in [0.15f32, 0.05f32].into_iter().enumerate() {
                if cancel.is_cancelled() {
                    return Err(AppError::InternalError(
                        "Subtitle job paused or cancelled".to_string(),
                    ));
                }
                match execute_single_attempt(
                    client,
                    slice,
                    prev_context,
                    next_context,
                    source_lang,
                    target_lang,
                    genre,
                    tag_shield_enabled,
                    temp,
                    attempt > 0,
                    cancel,
                    metrics,
                )
                .await
                {
                    Ok(mut outcomes) => {
                        if let Some(mut item) = outcomes.pop() {
                            item.self_healed = attempt > 0;
                            out_map.insert(item.cue_index, item);
                            return Ok(());
                        }
                    }
                    Err(AppError::InternalError(reason)) => {
                        last_err = Some(reason);
                    }
                    Err(fatal) => return Err(fatal),
                }
            }

            // Single cue exhausted its max 1 retry -> safe fallback keeping original text & tags.
            let warning_reason = last_err.unwrap_or_else(|| {
                "Model output violated JSON or Tag Shield placeholder structure after retry."
                    .to_string()
            });
            out_map.insert(
                cue.cue_index,
                CueOutcome {
                    cue_index: cue.cue_index,
                    translated_text: cue.original_text.clone(),
                    status: "warning",
                    warning_msg: Some(format!(
                        "Kept original text due to placeholder/format mismatch: {warning_reason}"
                    )),
                    self_healed: true,
                },
            );
            return Ok(());
        }

        // Batch len >= 2: Attempt 1 (near-deterministic temp 0.05 for genre consistency)
        match execute_single_attempt(
            client,
            slice,
            prev_context,
            next_context,
            source_lang,
            target_lang,
            genre,
            tag_shield_enabled,
            0.05,
            false,
            cancel,
            metrics,
        )
        .await
        {
            Ok(outcomes) => {
                for item in outcomes {
                    out_map.insert(item.cue_index, item);
                }
                return Ok(());
            }
            Err(AppError::InternalError(_)) => {}
            Err(fatal) => return Err(fatal),
        }

        // Batch Retry once at temperature 0.0 before splitting (only on outer batch).
        if allow_initial_batch_retry {
            if cancel.is_cancelled() {
                return Err(AppError::InternalError(
                    "Subtitle job paused or cancelled".to_string(),
                ));
            }
            match execute_single_attempt(
                client,
                slice,
                prev_context,
                next_context,
                source_lang,
                target_lang,
                genre,
                tag_shield_enabled,
                0.0,
                true,
                cancel,
                metrics,
            )
            .await
            {
                Ok(outcomes) => {
                    for mut item in outcomes {
                        item.self_healed = true;
                        out_map.insert(item.cue_index, item);
                    }
                    return Ok(());
                }
                Err(AppError::InternalError(_)) => {}
                Err(fatal) => return Err(fatal),
            }
        }

        // Level 3: Recursive Binary Split Fallback
        let mid = slice.len() / 2;
        translate_slice_bounded(
            client,
            &slice[..mid],
            prev_context,
            next_context,
            source_lang,
            target_lang,
            genre,
            tag_shield_enabled,
            false,
            cancel,
            metrics,
            out_map,
        )
        .await?;

        // Thread successful left-side cues into trailing context if available, else keep prev_context.
        translate_slice_bounded(
            client,
            &slice[mid..],
            prev_context,
            next_context,
            source_lang,
            target_lang,
            genre,
            tag_shield_enabled,
            false,
            cancel,
            metrics,
            out_map,
        )
        .await?;

        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
async fn execute_single_attempt<C: BatchCompletionClient>(
    client: &C,
    slice: &[PreparedCue],
    prev_context: &[(String, String)],
    next_context: &[String],
    source_lang: &str,
    target_lang: &str,
    genre: Genre,
    tag_shield_enabled: bool,
    temperature: f32,
    strict_retry_mode: bool,
    cancel: &CancellationToken,
    metrics: &mut MetricsAccumulator,
) -> AppResult<Vec<CueOutcome>> {
    let messages = build_batch_messages(
        slice,
        prev_context,
        next_context,
        source_lang,
        target_lang,
        genre,
        tag_shield_enabled,
        strict_retry_mode,
    );
    let max_tokens = ((slice.len() * 110) + 128).clamp(256, 3072) as u32;

    let schema_req = ChatCompletionRequest {
        messages: messages.clone(),
        temperature,
        max_tokens: Some(max_tokens),
        response_format: Some(subtitle_batch_json_schema()),
        disable_thinking: true,
    };

    let resp = tokio::select! {
        _ = cancel.cancelled() => {
            return Err(AppError::InternalError("Subtitle job paused or cancelled".to_string()));
        }
        res = client.complete_json(schema_req) => {
            match res {
                Ok(r) => r,
                Err(AppError::HttpError(_)) => {
                    // Fallback for custom BYOM models whose chat templates reject json_schema grammar.
                    let fallback_req = ChatCompletionRequest {
                        messages,
                        temperature,
                        max_tokens: Some(max_tokens),
                        response_format: None,
                        disable_thinking: true,
                    };
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            return Err(AppError::InternalError("Subtitle job paused or cancelled".to_string()));
                        }
                        r2 = client.complete_json(fallback_req) => r2?
                    }
                }
                Err(e) => return Err(e),
            }
        }
    };

    metrics.record(resp.ttft_ms, resp.tok_per_sec);
    validate_and_restore_batch(slice, &resp.content, tag_shield_enabled)
}

fn validate_and_restore_batch(
    slice: &[PreparedCue],
    raw_content: &str,
    tag_shield_enabled: bool,
) -> AppResult<Vec<CueOutcome>> {
    let payload = parse_batch_json_payload(raw_content)?;
    if payload.items.len() != slice.len() {
        return Err(AppError::InternalError(format!(
            "Batch item count mismatch: expected {}, got {}",
            slice.len(),
            payload.items.len()
        )));
    }

    let mut by_id: HashMap<i64, String> = HashMap::with_capacity(payload.items.len());
    for item in payload.items {
        if by_id.insert(item.id, item.text).is_some() {
            return Err(AppError::InternalError(format!(
                "Duplicate cue id {} in model JSON response",
                item.id
            )));
        }
    }

    let mut outcomes = Vec::with_capacity(slice.len());
    for cue in slice {
        let raw_translated = by_id.remove(&cue.cue_index).ok_or_else(|| {
            AppError::InternalError(format!("Missing cue id {} in model response", cue.cue_index))
        })?;

        let trimmed_clean = raw_translated.trim();
        let final_text = if tag_shield_enabled {
            if let Some(shield) = cue.shield.as_ref() {
                restore_tags(shield, trimmed_clean).map_err(|e| {
                    AppError::InternalError(format!("Cue #{}: {e}", cue.cue_index))
                })?
            } else {
                trimmed_clean.to_string()
            }
        } else {
            trimmed_clean.to_string()
        };

        outcomes.push(CueOutcome {
            cue_index: cue.cue_index,
            translated_text: final_text,
            status: "done",
            warning_msg: None,
            self_healed: false,
        });
    }

    Ok(outcomes)
}

fn parse_batch_json_payload(raw: &str) -> AppResult<BatchJsonPayload> {
    let cleaned = strip_think_tags(raw);
    let trimmed = cleaned.trim();
    if let Ok(parsed) = serde_json::from_str::<BatchJsonPayload>(trimmed) {
        return Ok(parsed);
    }

    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            if let Ok(parsed) = serde_json::from_str::<BatchJsonPayload>(&trimmed[start..=end]) {
                return Ok(parsed);
            }
        }
    }

    Err(AppError::InternalError(
        "Model did not return valid subtitle batch JSON".to_string(),
    ))
}

#[allow(clippy::too_many_arguments)]
fn build_batch_messages(
    slice: &[PreparedCue],
    prev_context: &[(String, String)],
    next_context: &[String],
    source_lang: &str,
    target_lang: &str,
    genre: Genre,
    tag_shield_enabled: bool,
    strict_retry_mode: bool,
) -> Vec<ChatMessage> {
    let target_info = language_info(target_lang);
    let source_label = if source_lang == "auto" {
        "automatically detected source language"
    } else {
        language_name(source_lang)
    };
    let genre_guide = genre.prompt_instruction_for_lang(target_lang);

    let mut system = if target_lang == "id" {
        format!(
            "Kamu adalah penerjemah subtitle profesional ({source_label} -> Bahasa Indonesia). {genre_guide}\n\
             Terjemahkan SETIAP item pada JSON input ke Bahasa Indonesia yang ringkas, alami dibaca di layar, dan setia konteks.\n\
             Kembalikan HANYA objek JSON valid berbentuk `{{\"items\": [{{\"id\": <integer>, \"text\": \"...\"}}]}}` \
             dengan jumlah elemen serta urutan `id` yang SAMA PERSIS seperti input."
        )
    } else {
        format!(
            "You are a professional subtitle translator ({source_label} -> {} ({})). {genre_guide}\n\
             Translate EVERY item in the input JSON into concise, natural {} subtitles faithful to dialogue flow and timing.\n\
             Return ONLY a valid JSON object shaped `{{\"items\": [{{\"id\": <integer>, \"text\": \"...\"}}]}}` \
             with the EXACT same item count and `id` sequence as the input.",
            target_info.name, target_info.native_name, target_info.name
        )
    };

    if tag_shield_enabled {
        system.push_str(
            "\nIMPORTANT (Tag Shield): Never delete, rename, duplicate, or invent XML placeholders like `<t1>`, `</t1>`, `<t1/>`, `<t2>`, etc. \
             Keep placeholders wrapping the matching translated span accurately.",
        );
    }

    if strict_retry_mode {
        system.push_str(
            "\nSTRICT RETRY WARNING: Previous attempt failed ID or `<tN>` placeholder verification. \
             Ensure all `id` values and every `<tN>` / `<tN/>` placeholder remain 100% intact.",
        );
    }

    // Collect deduplicated glossary terms matched anywhere inside this batch.
    let mut glossary_lines = Vec::new();
    let mut seen_terms = HashSet::new();
    for cue in slice {
        for m in &cue.glossary_matches {
            let key = if m.case_sensitive {
                m.source_term.clone()
            } else {
                m.source_term.to_lowercase()
            };
            if seen_terms.insert(key) {
                glossary_lines.push(format!("- \"{}\" => \"{}\"", m.source_term, m.target_term));
            }
        }
    }

    if !glossary_lines.is_empty() {
        system.push_str("\n\nEnforce these glossary translations consistently:\n");
        system.push_str(&glossary_lines.join("\n"));
    }

    if !prev_context.is_empty() {
        system.push_str("\n\nPrevious 1-3 dialogue lines (reference ONLY for pronoun/tone consistency; DO NOT include in output JSON):\n");
        for (idx, (src, dst)) in prev_context.iter().take(3).enumerate() {
            system.push_str(&format!("{}. SRC: {} | DST: {}\n", idx + 1, src, dst));
        }
    }

    if !next_context.is_empty() {
        system.push_str("\n\nUpcoming 1-2 dialogue lines (lookahead reference ONLY for sentence flow; DO NOT include in output JSON):\n");
        for (idx, src) in next_context.iter().take(2).enumerate() {
            system.push_str(&format!("{}. SRC: {}\n", idx + 1, src));
        }
    }

    let payload = BatchJsonPayload {
        items: slice
            .iter()
            .map(|c| BatchPromptItem {
                id: c.cue_index,
                text: c.source_for_prompt.clone(),
            })
            .collect(),
    };
    let user_content = serde_json::to_string(&payload).unwrap_or_else(|_| "{\"items\":[]}".to_string());

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system,
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_content,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::llama_client::LlamaClient;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn read_http_request(socket: &mut tokio::net::TcpStream) -> String {
        let mut buf = Vec::with_capacity(4096);
        let mut chunk = [0u8; 2048];
        let mut content_length = 0usize;
        let mut header_end = None;

        loop {
            let n = socket.read(&mut chunk).await.expect("read socket");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if header_end.is_none() {
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = Some(pos + 4);
                    let headers = String::from_utf8_lossy(&buf[..pos]);
                    for line in headers.lines() {
                        if let Some((k, v)) = line.split_once(':') {
                            if k.trim().eq_ignore_ascii_case("content-length") {
                                content_length = v.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                }
            }
            if let Some(end) = header_end {
                if buf.len() >= end + content_length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&buf).to_string()
    }

    #[test]
    fn maps_context_window_to_expected_batch_sizes() {
        assert_eq!(batch_size_for_ctx(2048), 6);
        assert_eq!(batch_size_for_ctx(4096), 12);
        assert_eq!(batch_size_for_ctx(8192), 18);
    }

    #[tokio::test]
    async fn bounded_self_healing_splits_corrupt_batch_and_flags_bad_single_cue() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback mock server");
        let port = listener.local_addr().unwrap().port();
        let request_count = Arc::new(AtomicUsize::new(0));
        let request_count_srv = Arc::clone(&request_count);

        let server = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let idx = request_count_srv.fetch_add(1, Ordering::SeqCst);
                let raw_req = read_http_request(&mut socket).await;
                let body_start = raw_req.find("\r\n\r\n").map(|p| p + 4).unwrap_or(0);
                let req_json: serde_json::Value =
                    serde_json::from_str(&raw_req[body_start..]).unwrap_or_default();

                // Extract input cue IDs from user prompt JSON
                let user_msg = req_json["messages"][1]["content"].as_str().unwrap_or("{}");
                let user_payload: BatchJsonPayload =
                    serde_json::from_str(user_msg).unwrap_or(BatchJsonPayload { items: vec![] });

                let assistant_json = if user_payload.items.len() >= 2 {
                    // Simulate persistent batch mismatch (missing id on attempt 0 & 1) to force binary split
                    serde_json::json!({
                        "items": [{ "id": 999, "text": "Broken batch" }]
                    })
                } else {
                    let item = &user_payload.items[0];
                    if item.id == 2 {
                        // Cue 2 intentionally drops `<t1>` even after retry -> must fall back with warning
                        serde_json::json!({
                            "items": [{ "id": 2, "text": "Ini tanpa tag" }]
                        })
                    } else {
                        serde_json::json!({
                            "items": [{ "id": item.id, "text": format!("Terjemahan #{}", item.id) }]
                        })
                    }
                };

                let chat_resp = serde_json::json!({
                    "choices": [{ "message": { "content": assistant_json.to_string() } }],
                    "usage": { "completion_tokens": 24 }
                })
                .to_string();
                let http_resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    chat_resp.len(),
                    chat_resp
                );
                let _ = socket.write_all(http_resp.as_bytes()).await;
                if idx > 15 {
                    break;
                }
            }
        });

        let client = LlamaClient::from_base_url(
            format!("http://127.0.0.1:{port}"),
            "test-key".to_string(),
        )
        .unwrap();

        let cue1 = SubtitleCueDto {
            job_id: "j1".to_string(),
            cue_index: 1,
            cue_type: "dialogue".to_string(),
            start_time: "00:00:01,000".to_string(),
            end_time: "00:00:02,000".to_string(),
            meta_json: None,
            glossary_matches_json: None,
            original_text: "{\\an8}Move now!".to_string(),
            translated_text: None,
            batch_number: None,
            status: "queued".to_string(),
            warning_msg: None,
        };
        let cue2 = SubtitleCueDto {
            cue_index: 2,
            original_text: "Go <i>there</i>!".to_string(),
            ..cue1.clone()
        };

        let prepared = vec![
            prepare_cue_for_pipeline(&cue1, true, &[]),
            prepare_cue_for_pipeline(&cue2, true, &[]),
        ];
        let cancel = CancellationToken::new();

        let out = translate_batch_with_healing(
            &client,
            &prepared,
            &[],
            &[],
            "en",
            "id",
            Genre::Anime,
            true,
            &cancel,
        )
        .await
        .expect("pipeline must heal gracefully without aborting");

        assert_eq!(out.outcomes.len(), 2);
        // Cue 1 succeeded via binary split and preserved leading {\an8}
        assert_eq!(out.outcomes[0].status, "done");
        assert_eq!(out.outcomes[0].translated_text, "{\\an8}Terjemahan #1");
        // Cue 2 exhausted max 1 retry and fell back safely to original text with warning
        assert_eq!(out.outcomes[1].status, "warning");
        assert_eq!(out.outcomes[1].translated_text, "Go <i>there</i>!");
        assert!(out.outcomes[1].warning_msg.is_some());
        // Total requests: 2 batch tries + 1 cue#1 + 2 cue#2 (initial + 1 retry) = 5 calls (strictly bounded)
        assert_eq!(request_count.load(Ordering::SeqCst), 5);

        server.abort();
    }
}
