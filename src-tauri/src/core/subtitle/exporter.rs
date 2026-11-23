use std::fs;
use std::io::Write;
use std::path::Path;

use uuid::Uuid;

use super::ass::serialize_ass;
use super::srt::serialize_srt;
use super::types::{CueMeta, CueType, ParsedSubtitleCue, SubtitleFormat};
use super::vtt::serialize_vtt;
use crate::core::hardware::disk;
use crate::db::models::{SubtitleCueDto, SubtitleJobRecord};
use crate::error::{AppError, AppResult};

/// Reconstructs the complete translated subtitle document from database state.
///
/// Export preserves all non-dialogue rows (`Comment:`, `\p1` drawings, WebVTT `STYLE`/`NOTE`
/// headers, and ASS `[Script Info]`, `[V4+ Styles]`, `[Fonts]`, `[Graphics]`).
pub fn serialize_subtitle_document(
    job: &SubtitleJobRecord,
    cues: &[SubtitleCueDto],
) -> AppResult<String> {
    let format = SubtitleFormat::from_str_lossy(&job.summary.format)?;
    let mut parsed_cues = Vec::with_capacity(cues.len());
    let mut translated_slices: Vec<Option<String>> = Vec::with_capacity(cues.len());

    for cue in cues {
        let meta: CueMeta = match cue.meta_json.as_deref() {
            Some(raw) if !raw.trim().is_empty() => {
                serde_json::from_str(raw).unwrap_or_default()
            }
            _ => CueMeta::default(),
        };

        let cue_type = CueType::from_str_lossy(&cue.cue_type);
        let parsed = ParsedSubtitleCue {
            cue_index: cue.cue_index,
            cue_type,
            start_time: cue.start_time.clone(),
            end_time: cue.end_time.clone(),
            meta,
            original_text: cue.original_text.clone(),
        };

        let effective_translation = match cue_type {
            CueType::Dialogue => Some(
                cue.translated_text
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| cue.original_text.clone()),
            ),
            CueType::Comment | CueType::Drawing => None,
        };

        parsed_cues.push(parsed);
        translated_slices.push(effective_translation);
    }

    let pairs = parsed_cues
        .iter()
        .zip(translated_slices.iter().map(|opt| opt.as_deref()));

    let output = match format {
        SubtitleFormat::Srt => serialize_srt(pairs),
        SubtitleFormat::Vtt => serialize_vtt(job.raw_header.as_deref(), pairs),
        SubtitleFormat::Ass => serialize_ass(
            job.raw_header.as_deref(),
            job.raw_footer.as_deref(),
            pairs,
        ),
    };

    Ok(output)
}

/// Writes exported subtitle content atomically via a temporary sibling file and atomic replace.
pub fn write_atomic_export(target_path: &Path, content: &str, include_utf8_bom: bool) -> AppResult<()> {
    let canonical_target = disk::validate_export_file_path(target_path)?;
    let parent = canonical_target.parent().ok_or_else(|| {
        AppError::InvalidPath("Export destination has no parent directory.".to_string())
    })?;

    let tmp_name = format!(".osmium_export_{}.tmp", Uuid::new_v4());
    let tmp_path = parent.join(tmp_name);

    let write_res = (|| -> AppResult<()> {
        let mut file = fs::File::create(&tmp_path)?;
        if include_utf8_bom {
            file.write_all(&[0xEF, 0xBB, 0xBF])?;
        }
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    if let Err(err) = fs::rename(&tmp_path, &canonical_target) {
        let _ = fs::remove_file(&tmp_path);
        return Err(AppError::IoError(format!(
            "Failed to finalize exported file '{}': {err}",
            canonical_target.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::subtitle::parse_subtitle_content;
    use crate::core::subtitle::types::SubtitleFormat;
    use crate::db::models::SubtitleJobSummary;

    #[test]
    fn exports_ass_round_trip_atomically() {
        let sample = "\
[Script Info]
Title: Export Test

[V4+ Styles]
Format: Name, Fontname, Fontsize
Style: Default,Arial,20

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Comment: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,Note stay
Dialogue: 0,0:00:01.00,0:00:03.00,Default,Hero,0,0,0,,{\\an8}Move out, team!

[Graphics]
filename: logo.png
";
        let doc = parse_subtitle_content(sample, SubtitleFormat::Ass, "UTF-8".to_string(), false)
            .expect("parse ass");
        let job = SubtitleJobRecord {
            summary: SubtitleJobSummary {
                id: "job-1".to_string(),
                file_name: "ep01.ass".to_string(),
                file_path: "E:/subs/ep01.ass".to_string(),
                file_size_bytes: sample.len() as i64,
                format: "ass".to_string(),
                genre: "anime".to_string(),
                glossary_id: None,
                tag_shield_enabled: true,
                model_id: "m1".to_string(),
                model_display_name: "Qwen3 8B".to_string(),
                status: "completed".to_string(),
                total_cues: 2,
                completed_cues: 2,
                self_healing_errors: 0,
                created_at: 0,
                updated_at: 0,
            },
            raw_header: doc.raw_header,
            raw_footer: doc.raw_footer,
            ass_format_cols: doc.ass_format_cols.map(|v| serde_json::to_string(&v).unwrap()),
        };

        let cue_dtos: Vec<SubtitleCueDto> = doc
            .cues
            .iter()
            .map(|c| SubtitleCueDto {
                job_id: "job-1".to_string(),
                cue_index: c.cue_index,
                cue_type: c.cue_type.as_str().to_string(),
                start_time: c.start_time.clone(),
                end_time: c.end_time.clone(),
                meta_json: Some(serde_json::to_string(&c.meta).unwrap()),
                glossary_matches_json: None,
                original_text: c.original_text.clone(),
                translated_text: if c.cue_type == CueType::Dialogue {
                    Some("{\\an8}Bergerak maju, tim!".to_string())
                } else {
                    None
                },
                batch_number: Some(1),
                status: if c.cue_type == CueType::Dialogue {
                    "done".to_string()
                } else {
                    "skipped".to_string()
                },
                warning_msg: None,
            })
            .collect();

        let rendered = serialize_subtitle_document(&job, &cue_dtos).expect("serialize");
        assert!(rendered.contains("Comment: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,Note stay"));
        assert!(rendered.contains("Dialogue: 0,0:00:01.00,0:00:03.00,Default,Hero,0,0,0,,{\\an8}Bergerak maju, tim!"));
        assert!(rendered.contains("[Graphics]"));

        let dir = std::env::temp_dir().join(format!("osmium_export_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("ep01.id.ass");
        write_atomic_export(&target, &rendered, false).expect("atomic export");
        let written = std::fs::read_to_string(&target).unwrap();
        assert_eq!(written, rendered);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
