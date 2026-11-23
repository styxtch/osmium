use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use crate::core::engine::llama_client::{ChatCompletionRequest, ChatCompletionResponse, LlamaClient};
use crate::core::subtitle::types::{CueType, GlossaryMatch, TagShieldResult};
use crate::db::models::SubtitleCueDto;
use crate::error::AppResult;

/// Async abstraction over `LlamaClient` so `batch_translator` can be tested with local mock
/// servers and scripted test doubles cleanly.
pub trait BatchCompletionClient: Send + Sync {
    fn complete_json<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn Future<Output = AppResult<ChatCompletionResponse>> + Send + 'a>>;
}

impl BatchCompletionClient for LlamaClient {
    fn complete_json<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn Future<Output = AppResult<ChatCompletionResponse>> + Send + 'a>> {
        Box::pin(async move { self.chat_completion_json(req).await })
    }
}

/// Prepared subtitle cue handed to `batch_translator`.
#[derive(Debug, Clone)]
pub struct PreparedCue {
    pub cue_index: i64,
    pub cue_type: CueType,
    pub original_text: String,
    pub source_for_prompt: String,
    pub shield: Option<TagShieldResult>,
    pub glossary_matches: Vec<GlossaryMatch>,
}

/// Result for a single cue produced by `translate_batch_with_healing`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CueOutcome {
    pub cue_index: i64,
    pub translated_text: String,
    pub status: &'static str,
    pub warning_msg: Option<String>,
    pub self_healed: bool,
}

/// Batch translation metrics & cue outcomes.
#[derive(Debug, Clone)]
pub struct BatchTranslationOutput {
    pub outcomes: Vec<CueOutcome>,
    pub avg_tok_per_sec: f64,
    pub avg_ttft_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleBatchStartedEvent {
    pub job_id: String,
    pub batch_number: i64,
    pub cue_indices: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleBatchCompletedEvent {
    pub job_id: String,
    pub batch_number: i64,
    pub cues: Vec<SubtitleCueDto>,
    pub completed_cues: i64,
    pub total_cues: i64,
    pub self_healing_errors: i64,
    pub status: String,
    pub tok_per_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleJobErrorEvent {
    pub job_id: String,
    pub code: String,
    pub message: String,
}
