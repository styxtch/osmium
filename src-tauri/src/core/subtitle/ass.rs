use super::tag_shield::contains_ass_drawing_command;
use super::types::{CueMeta, CueType, ParsedSubtitleCue, ParsedSubtitleDocument, SubtitleFormat};
use crate::error::{AppError, AppResult};

const DEFAULT_ASS_COLS: [&str; 10] = [
    "Layer", "Start", "End", "Style", "Name", "MarginL", "MarginR", "MarginV", "Effect", "Text",
];

/// Parses a normalized UTF-8 `.ass` / `.ssa` file while preserving `[Script Info]`,
/// `[V4+ Styles]`, `Format:`, `[Events]` order, `Comment:` lines, drawing lines (`\p1..9`),
/// commas inside dialogue text, and `[Fonts]`/`[Graphics]` footers.
pub fn parse_ass(
    text: &str,
    detected_encoding: String,
    had_bom: bool,
) -> AppResult<ParsedSubtitleDocument> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(events_idx) = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[Events]"))
    else {
        return Err(AppError::InvalidInput(
            "Invalid ASS/SSA file: missing '[Events]' section.".to_string(),
        ));
    };

    let mut header_lines: Vec<String> = lines[..=events_idx]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let mut cursor = events_idx + 1;

    // Preserve Format: line right below [Events] (skip leading blank lines between [Events] and Format:).
    let mut format_cols: Option<Vec<String>> = None;
    while cursor < lines.len() {
        let trimmed = lines[cursor].trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            header_lines.push(lines[cursor].to_string());
            cursor += 1;
            continue;
        }
        if is_section_header(trimmed) {
            break;
        }
        if let Some(cols_raw) = strip_prefix_ignore_ascii_case(trimmed, "Format:") {
            format_cols = Some(
                cols_raw
                    .split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect(),
            );
            header_lines.push(lines[cursor].to_string());
            cursor += 1;
        }
        break;
    }

    let resolved_cols = format_cols
        .clone()
        .unwrap_or_else(|| DEFAULT_ASS_COLS.iter().map(|s| (*s).to_string()).collect());
    let start_col = find_col_index(&resolved_cols, "Start").unwrap_or(1);
    let end_col = find_col_index(&resolved_cols, "End").unwrap_or(2);
    let text_col = find_col_index(&resolved_cols, "Text")
        .unwrap_or_else(|| resolved_cols.len().saturating_sub(1));

    let mut cues: Vec<ParsedSubtitleCue> = Vec::new();
    let mut footer_lines: Option<Vec<String>> = None;
    let mut cue_index = 1i64;

    while cursor < lines.len() {
        let line = lines[cursor];
        let trimmed = line.trim();

        if is_section_header(trimmed) {
            footer_lines = Some(lines[cursor..].iter().map(|s| (*s).to_string()).collect());
            break;
        }

        if let Some((prefix_keyword, after_keyword)) = split_event_kind(line) {
            if let Some(parsed) = parse_ass_event_row(
                prefix_keyword,
                after_keyword,
                start_col,
                end_col,
                text_col,
            ) {
                let cue_type = if parsed.is_comment {
                    CueType::Comment
                } else if contains_ass_drawing_command(&parsed.dialogue_text) {
                    CueType::Drawing
                } else {
                    CueType::Dialogue
                };

                cues.push(ParsedSubtitleCue {
                    cue_index,
                    cue_type,
                    start_time: parsed.start_time,
                    end_time: parsed.end_time,
                    meta: CueMeta {
                        ass_raw_prefix: Some(parsed.raw_prefix),
                        ..Default::default()
                    },
                    original_text: parsed.dialogue_text,
                });
                cue_index += 1;
                cursor += 1;
                continue;
            }
        }

        // Blank lines or semicolon comments inside [Events] preserved verbatim in order.
        if !trimmed.is_empty() || !cues.is_empty() {
            cues.push(ParsedSubtitleCue {
                cue_index,
                cue_type: CueType::Comment,
                start_time: String::new(),
                end_time: String::new(),
                meta: CueMeta {
                    ass_raw_prefix: Some("__ASS_RAW_LINE__".to_string()),
                    ..Default::default()
                },
                original_text: line.to_string(),
            });
            cue_index += 1;
        } else {
            header_lines.push(line.to_string());
        }
        cursor += 1;
    }

    if cues.is_empty() {
        return Err(AppError::InvalidInput(
            "ASS/SSA file contains no events inside '[Events]'.".to_string(),
        ));
    }

    Ok(ParsedSubtitleDocument {
        format: SubtitleFormat::Ass,
        detected_encoding,
        had_bom,
        raw_header: Some(header_lines.join("\n")),
        raw_footer: footer_lines.map(|f| f.join("\n")),
        ass_format_cols: Some(resolved_cols),
        cues,
    })
}

/// Serializes cues back into canonical `.ass` format.
pub fn serialize_ass<'a, I>(
    raw_header: Option<&str>,
    raw_footer: Option<&str>,
    cues: I,
) -> String
where
    I: IntoIterator<Item = (&'a ParsedSubtitleCue, Option<&'a str>)>,
{
    let mut lines: Vec<String> = Vec::new();
    if let Some(header) = raw_header {
        lines.push(header.trim_end_matches('\n').to_string());
    }

    for (cue, translated_opt) in cues {
        let prefix = cue.meta.ass_raw_prefix.as_deref().unwrap_or("Dialogue: 0,,,Default,,0,0,0,,");
        if prefix == "__ASS_RAW_LINE__" {
            lines.push(cue.original_text.clone());
            continue;
        }

        let body = match cue.cue_type {
            CueType::Dialogue => translated_opt.unwrap_or(&cue.original_text),
            CueType::Comment | CueType::Drawing => &cue.original_text,
        };
        lines.push(format!("{prefix}{body}"));
    }

    if let Some(footer) = raw_footer {
        if !footer.is_empty() {
            lines.push(footer.trim_end_matches('\n').to_string());
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

struct ParsedEventRow {
    is_comment: bool,
    start_time: String,
    end_time: String,
    raw_prefix: String,
    dialogue_text: String,
}

fn parse_ass_event_row(
    prefix_with_colon: &str,
    after_colon: &str,
    start_col: usize,
    end_col: usize,
    text_col: usize,
) -> Option<ParsedEventRow> {
    // Find the byte index right after the `text_col`-th comma so dialogue commas are never split.
    let mut comma_positions = Vec::with_capacity(text_col);
    for (idx, b) in after_colon.bytes().enumerate() {
        if b == b',' {
            comma_positions.push(idx);
            if comma_positions.len() == text_col {
                break;
            }
        }
    }
    if comma_positions.len() < text_col {
        return None;
    }

    let split_comma_pos = comma_positions[text_col - 1];
    let before_text = &after_colon[..split_comma_pos];
    let fields: Vec<&str> = before_text.split(',').collect();

    let start_time = fields.get(start_col).map(|s| s.trim()).unwrap_or("").to_string();
    let end_time = fields.get(end_col).map(|s| s.trim()).unwrap_or("").to_string();

    let raw_prefix = format!("{prefix_with_colon}{},", &after_colon[..split_comma_pos]);
    let dialogue_text = after_colon[split_comma_pos + 1..].to_string();
    let is_comment = prefix_with_colon
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("comment:");

    Some(ParsedEventRow {
        is_comment,
        start_time,
        end_time,
        raw_prefix,
        dialogue_text,
    })
}

fn split_event_kind(line: &str) -> Option<(&str, &str)> {
    let trimmed_start = line.trim_start();
    let leading_ws_len = line.len() - trimmed_start.len();
    for kw in ["Dialogue:", "Comment:"] {
        if let Some(after) = strip_prefix_ignore_ascii_case(trimmed_start, kw) {
            let split_idx = leading_ws_len + kw.len();
            return Some((&line[..split_idx], after));
        }
    }
    None
}

fn strip_prefix_ignore_ascii_case<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() < prefix.len() || !input.is_char_boundary(prefix.len()) {
        return None;
    }
    if input[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&input[prefix.len()..])
    } else {
        None
    }
}

fn is_section_header(trimmed: &str) -> bool {
    trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 3
}

fn find_col_index(cols: &[String], target: &str) -> Option<usize> {
    cols.iter().position(|c| c.eq_ignore_ascii_case(target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_ass_styles_commas_comments_drawings_and_fonts_footer() {
        let sample = "\
[Script Info]
Title: Osmium Test
ScriptType: v4.00+

[V4+ Styles]
Format: Name, Fontname, Fontsize
Style: Default,Arial,20

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Comment: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,Translator note: keep untouched
Dialogue: 0,0:00:01.00,0:00:03.50,Default,Alice,0,0,0,,Wait, Bob, what is {\\i1}that{\\i0}?
Dialogue: 1,0:00:03.50,0:00:05.00,Default,,0,0,0,,{\\an7\\p1}m 0 0 l 50 0 50 50

[Fonts]
fontname: custom_arial.ttf
AAEBAQ==
";
        let doc = parse_ass(sample, "UTF-8".to_string(), false).expect("parse ass");
        assert_eq!(doc.cues.len(), 4);
        assert_eq!(doc.cues[0].cue_type, CueType::Comment);
        assert_eq!(doc.cues[1].cue_type, CueType::Dialogue);
        assert_eq!(
            doc.cues[1].original_text,
            "Wait, Bob, what is {\\i1}that{\\i0}?"
        );
        assert_eq!(doc.cues[2].cue_type, CueType::Drawing);
        assert!(doc.raw_footer.as_deref().unwrap().contains("[Fonts]"));

        let serialized = serialize_ass(
            doc.raw_header.as_deref(),
            doc.raw_footer.as_deref(),
            doc.cues.iter().map(|c| (c, None)),
        );
        assert_eq!(serialized, sample);
    }
}
