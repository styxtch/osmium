pub mod ass;
pub mod encoding;
pub mod exporter;
pub mod glossary_matcher;
pub mod srt;
pub mod tag_shield;
pub mod types;
pub mod vtt;

use std::fs;
use std::path::Path;

use crate::core::hardware::disk;
use crate::error::{AppError, AppResult};
use types::{ParsedSubtitleDocument, SubtitleFormat};

const MAX_SUBTITLE_FILE_BYTES: u64 = 50 * 1024 * 1024;

/// Reads a subtitle file directly from disk (never copied to AppData) and decodes + parses it.
pub fn parse_subtitle_file(path: &Path) -> AppResult<(ParsedSubtitleDocument, i64)> {
    let canonical = disk::validate_readable_file(path)?;
    let meta = fs::metadata(&canonical)?;
    let file_size = meta.len();
    if file_size > MAX_SUBTITLE_FILE_BYTES {
        return Err(AppError::InvalidInput(
            "Subtitle file exceeds the 50 MB safety limit.".to_string(),
        ));
    }

    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    let format = SubtitleFormat::from_extension(ext)?;

    let bytes = fs::read(&canonical)?;
    let decoded = encoding::decode_subtitle_bytes(&bytes)?;
    let doc = parse_subtitle_content(
        &decoded.text,
        format,
        decoded.detected_encoding,
        decoded.had_bom,
    )?;

    Ok((doc, file_size as i64))
}

/// Parses already-decoded UTF-8 subtitle content using the target container format.
pub fn parse_subtitle_content(
    text: &str,
    format: SubtitleFormat,
    detected_encoding: String,
    had_bom: bool,
) -> AppResult<ParsedSubtitleDocument> {
    match format {
        SubtitleFormat::Srt => srt::parse_srt(text, detected_encoding, had_bom),
        SubtitleFormat::Vtt => vtt::parse_vtt(text, detected_encoding, had_bom),
        SubtitleFormat::Ass => ass::parse_ass(text, detected_encoding, had_bom),
    }
}
