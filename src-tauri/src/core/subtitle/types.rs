use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Supported subtitle container formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
    Srt,
    Vtt,
    Ass,
}

impl SubtitleFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Ass => "ass",
        }
    }

    pub fn from_extension(ext: &str) -> AppResult<Self> {
        match ext.trim().to_ascii_lowercase().as_str() {
            "srt" => Ok(Self::Srt),
            "vtt" => Ok(Self::Vtt),
            "ass" | "ssa" => Ok(Self::Ass),
            other => Err(AppError::InvalidInput(format!(
                "Unsupported subtitle format '.{other}'. Supported formats: .srt, .vtt, .ass."
            ))),
        }
    }

    pub fn from_str_lossy(raw: &str) -> AppResult<Self> {
        Self::from_extension(raw.trim_start_matches('.'))
    }
}

/// Classification of an individual subtitle line/cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CueType {
    Dialogue,
    Comment,
    Drawing,
}

impl CueType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dialogue => "dialogue",
            Self::Comment => "comment",
            Self::Drawing => "drawing",
        }
    }

    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "comment" => Self::Comment,
            "drawing" => Self::Drawing,
            _ => Self::Dialogue,
        }
    }
}

/// Format-specific metadata stored verbatim in `subtitle_cues.meta_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CueMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srt_sequence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vtt_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vtt_settings: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ass_prefix_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ass_raw_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leading_tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trailing_tags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_tags: Option<Vec<String>>,
}

/// A parsed cue ready for SQLite insertion or deterministic round-trip serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSubtitleCue {
    pub cue_index: i64,
    pub cue_type: CueType,
    pub start_time: String,
    pub end_time: String,
    pub meta: CueMeta,
    pub original_text: String,
}

/// Complete parsed subtitle file with preserved headers, footers, styles, and cues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSubtitleDocument {
    pub format: SubtitleFormat,
    pub detected_encoding: String,
    pub had_bom: bool,
    pub raw_header: Option<String>,
    pub raw_footer: Option<String>,
    pub ass_format_cols: Option<Vec<String>>,
    pub cues: Vec<ParsedSubtitleCue>,
}

/// Mapping for a single inline markup placeholder (`<tN>...</tN>` or `<tN/>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineTagMapping {
    pub id: usize,
    pub kind: InlineTagKind,
    pub open_tag: String,
    pub close_tag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InlineTagKind {
    Paired,
    SelfClosing,
}

/// Output of Rust Tag Shield before sending clean text to the LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagShieldResult {
    pub clean_text: String,
    pub leading_tags: String,
    pub trailing_tags: String,
    pub inline_mappings: Vec<InlineTagMapping>,
    pub display_tags: Vec<String>,
    pub is_drawing: bool,
}

/// Matched glossary entry attached to a subtitle cue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryMatch {
    pub source_term: String,
    pub target_term: String,
    pub case_sensitive: bool,
}
