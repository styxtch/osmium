use super::types::{CueMeta, CueType, ParsedSubtitleCue, ParsedSubtitleDocument, SubtitleFormat};
use crate::error::{AppError, AppResult};

/// Parses a normalized UTF-8 WebVTT (`.vtt`) document while preserving `WEBVTT`, `STYLE`, `REGION`,
/// and `NOTE` blocks alongside cue identifiers and cue settings.
pub fn parse_vtt(
    text: &str,
    detected_encoding: String,
    had_bom: bool,
) -> AppResult<ParsedSubtitleDocument> {
    let trimmed = text.trim();
    if !trimmed.starts_with("WEBVTT") {
        return Err(AppError::InvalidInput(
            "Invalid WebVTT file: missing 'WEBVTT' header.".to_string(),
        ));
    }

    let raw_blocks = split_vtt_blocks(text);
    if raw_blocks.is_empty() {
        return Err(AppError::InvalidInput(
            "WebVTT file contains no header or cue blocks.".to_string(),
        ));
    }

    let mut header_blocks: Vec<String> = Vec::new();
    let mut cues: Vec<ParsedSubtitleCue> = Vec::new();
    let mut seen_first_cue = false;
    let mut cue_index = 1i64;

    for block in raw_blocks {
        let lines: Vec<&str> = block.lines().collect();
        if lines.is_empty() {
            continue;
        }

        if !seen_first_cue && !block_has_timing_line(&lines) {
            header_blocks.push(block);
            continue;
        }

        // Handle mid-file NOTE / STYLE / REGION blocks verbatim as non-dialogue comment cues.
        if !block_has_timing_line(&lines) {
            cues.push(ParsedSubtitleCue {
                cue_index,
                cue_type: CueType::Comment,
                start_time: String::new(),
                end_time: String::new(),
                meta: CueMeta {
                    vtt_identifier: Some("__VTT_RAW_BLOCK__".to_string()),
                    ..Default::default()
                },
                original_text: block,
            });
            cue_index += 1;
            continue;
        }

        seen_first_cue = true;
        let (vtt_identifier, timing_idx) = if lines[0].contains("-->") {
            (None, 0usize)
        } else {
            (Some(lines[0].to_string()), 1usize)
        };

        let (start_time, end_time, vtt_settings) =
            parse_vtt_timing_line(lines[timing_idx], cue_index)?;
        let original_text = if timing_idx + 1 < lines.len() {
            lines[timing_idx + 1..].join("\n")
        } else {
            String::new()
        };

        let cue_type = if original_text.trim().is_empty() {
            CueType::Comment
        } else {
            CueType::Dialogue
        };

        cues.push(ParsedSubtitleCue {
            cue_index,
            cue_type,
            start_time,
            end_time,
            meta: CueMeta {
                vtt_identifier,
                vtt_settings,
                ..Default::default()
            },
            original_text,
        });
        cue_index += 1;
    }

    if cues.is_empty() {
        return Err(AppError::InvalidInput(
            "WebVTT file contains no subtitle cues.".to_string(),
        ));
    }

    let raw_header = Some(header_blocks.join("\n\n"));

    Ok(ParsedSubtitleDocument {
        format: SubtitleFormat::Vtt,
        detected_encoding,
        had_bom,
        raw_header,
        raw_footer: None,
        ass_format_cols: None,
        cues,
    })
}

/// Serializes cues back into canonical `.vtt` format preserving `WEBVTT`, `STYLE`, `REGION`, `NOTE`,
/// cue identifiers, and cue positioning settings.
pub fn serialize_vtt<'a, I>(raw_header: Option<&str>, cues: I) -> String
where
    I: IntoIterator<Item = (&'a ParsedSubtitleCue, Option<&'a str>)>,
{
    let mut sections: Vec<String> = Vec::new();
    let header = raw_header
        .map(str::trim_end)
        .filter(|h| !h.is_empty())
        .unwrap_or("WEBVTT");
    sections.push(header.to_string());

    for (cue, translated_opt) in cues {
        if cue.meta.vtt_identifier.as_deref() == Some("__VTT_RAW_BLOCK__") {
            sections.push(cue.original_text.clone());
            continue;
        }

        let mut block_lines = Vec::new();
        if let Some(ident) = cue.meta.vtt_identifier.as_deref() {
            if !ident.is_empty() {
                block_lines.push(ident.to_string());
            }
        }

        let mut timing = format!("{} --> {}", cue.start_time, cue.end_time);
        if let Some(settings) = cue.meta.vtt_settings.as_deref() {
            if !settings.is_empty() {
                timing.push(' ');
                timing.push_str(settings);
            }
        }
        block_lines.push(timing);

        let body = match cue.cue_type {
            CueType::Dialogue => translated_opt.unwrap_or(&cue.original_text),
            CueType::Comment | CueType::Drawing => &cue.original_text,
        };
        if !body.is_empty() {
            block_lines.push(body.to_string());
        }
        sections.push(block_lines.join("\n"));
    }

    let mut out = sections.join("\n\n");
    out.push('\n');
    out
}

fn split_vtt_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

fn block_has_timing_line(lines: &[&str]) -> bool {
    if lines.is_empty() {
        return false;
    }
    if lines[0].starts_with("NOTE")
        || lines[0].starts_with("STYLE")
        || lines[0].starts_with("REGION")
        || lines[0].starts_with("WEBVTT")
    {
        return false;
    }
    lines[0].contains("-->") || (lines.len() >= 2 && lines[1].contains("-->"))
}

fn parse_vtt_timing_line(line: &str, cue_index: i64) -> AppResult<(String, String, Option<String>)> {
    let Some((left, right)) = line.split_once("-->") else {
        return Err(AppError::InvalidInput(format!(
            "Invalid WebVTT timestamp in cue #{cue_index}."
        )));
    };
    let start = left.trim();
    let right_trimmed = right.trim();
    if start.is_empty() || right_trimmed.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Incomplete WebVTT timestamp in cue #{cue_index}."
        )));
    }

    let (end, settings) = match right_trimmed.find(char::is_whitespace) {
        Some(pos) => (
            right_trimmed[..pos].trim(),
            Some(right_trimmed[pos..].trim().to_string()).filter(|s| !s.is_empty()),
        ),
        None => (right_trimmed, None),
    };

    Ok((start.to_string(), end.to_string(), settings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_vtt_headers_styles_settings_and_note() {
        let sample = "\
WEBVTT - Episode 01

STYLE
::cue { color: white; }

cue-1
00:00:01.000 --> 00:00:03.500 align:start position:10%
<v Narrator>Welcome to Neo Tokyo.</v>

NOTE mid-episode comment

00:00:05.000 --> 00:00:07.000
Let's move out.
";
        let doc = parse_vtt(sample, "UTF-8".to_string(), false).expect("parse vtt");
        assert_eq!(doc.cues.len(), 3);
        assert_eq!(doc.cues[0].meta.vtt_identifier.as_deref(), Some("cue-1"));
        assert_eq!(
            doc.cues[0].meta.vtt_settings.as_deref(),
            Some("align:start position:10%")
        );
        assert_eq!(doc.cues[1].cue_type, CueType::Comment);

        let serialized = serialize_vtt(doc.raw_header.as_deref(), doc.cues.iter().map(|c| (c, None)));
        assert_eq!(serialized, sample);
    }
}
