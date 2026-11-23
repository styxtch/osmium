use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub display_mode: String,
    pub default_model_dir: String,
    pub default_export_dir: String,
    pub backend_override: String,
    pub ctx_size_override: String,
    pub gpu_layers_override: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledModel {
    pub id: String,
    pub display_name: String,
    pub tier: String,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub sha256: Option<String>,
    pub quantization: Option<String>,
    pub is_byom: bool,
    pub block_count: Option<i64>,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadTask {
    pub model_id: String,
    pub download_url: String,
    pub target_path: String,
    pub part_path: String,
    pub expected_sha256: String,
    pub etag: Option<String>,
    pub total_bytes: i64,
    pub bytes_downloaded: i64,
    pub status: String,
    pub error_msg: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextHistoryItem {
    pub id: String,
    pub source_text: String,
    pub translated_text: String,
    pub tone: String,
    pub model_id: String,
    pub model_display_name: String,
    pub ttft_ms: Option<i64>,
    pub tok_per_sec: Option<f64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Glossary {
    pub id: String,
    pub name: String,
    pub entry_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlossaryEntry {
    pub id: String,
    pub glossary_id: String,
    pub source_term: String,
    pub target_term: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubtitleJobSummary {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub format: String,
    pub genre: String,
    pub glossary_id: Option<String>,
    pub tag_shield_enabled: bool,
    pub model_id: String,
    pub model_display_name: String,
    pub status: String,
    pub total_cues: i64,
    pub completed_cues: i64,
    pub self_healing_errors: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubtitleJobRecord {
    pub summary: SubtitleJobSummary,
    pub raw_header: Option<String>,
    pub raw_footer: Option<String>,
    pub ass_format_cols: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubtitleCueDto {
    pub job_id: String,
    pub cue_index: i64,
    pub cue_type: String,
    pub start_time: String,
    pub end_time: String,
    pub meta_json: Option<String>,
    pub glossary_matches_json: Option<String>,
    pub original_text: String,
    pub translated_text: Option<String>,
    pub batch_number: Option<i64>,
    pub status: String,
    pub warning_msg: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSubtitleCue {
    pub cue_index: i64,
    pub cue_type: String,
    pub start_time: String,
    pub end_time: String,
    pub meta_json: Option<String>,
    pub glossary_matches_json: Option<String>,
    pub original_text: String,
    pub translated_text: Option<String>,
    pub status: String,
}
