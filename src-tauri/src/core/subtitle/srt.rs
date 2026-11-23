use super::types::{CueMeta, CueType, ParsedSubtitleCue, ParsedSubtitleDocument, SubtitleFormat};
use crate::error::{AppError, AppResult};

/// Parses a normalized UTF-8 `.srt` document into cues.
pub fn parse_srt(
    text: &str,
    detected_encoding: String,
    had_bom: bool,
) -> AppResult<ParsedSubtitleDocument> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            "SRT file contains no subtitle cues.".to_string(),
        ));
    }

    let mut cues = Vec::new();
    let mut lines = text.lines().peekable();
    let mut cue_index = 1i64;

    while lines.peek().is_some() {
        // Skip empty separator lines.
        while let Some(line) = lines.peek() {
            if line.trim().is_empty() {
                lines.next();
            } else {
                break;
            }
        }
        if lines.peek().is_none() {
            break;
        }

        // Gather one non-empty block until blank line.
        let mut block: Vec<&str> = Vec::new();
        while let Some(line) = lines.next() {
            if line.trim().is_empty() {
                break;
            }
            block.push(line);
        }

        if block.is_empty() {
            continue;
        }

        // Determine if line 0 is sequence number or direct timing line.
        let (srt_sequence, timing_idx) = if block.len() >= 2 && block[1].contains("-->") {
            (Some(block[0].trim().to_string()), 1usize)
        } else if block[0].contains("-->") {
            (None, 0usize)
        } else {
            return Err(AppError::InvalidInput(format!(
                "Malformed SRT cue near cue #{cue_index}: missing '-->' timestamp separator."
            )));
        };

        let (start_time, end_time) = parse_timing_line(block[timing_idx], cue_index)?;
        let original_text = if timing_idx + 1 < block.len() {
            block[timing_idx + 1..].join("\n")
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
                srt_sequence,
                ..Default::default()
            },
            original_text,
        });
        cue_index += 1;
    }

    if cues.is_empty() {
        return Err(AppError::InvalidInput(
            "SRT file contains no valid subtitle cues.".to_string(),
        ));
    }

    Ok(ParsedSubtitleDocument {
        format: SubtitleFormat::Srt,
        detected_encoding,
        had_bom,
        raw_header: None,
        raw_footer: None,
        ass_format_cols: None,
        cues,
    })
}

/// Serializes cues back into canonical `.srt` format.
pub fn serialize_srt<'a, I>(cues: I) -> String
where
    I: IntoIterator<Item = (&'a ParsedSubtitleCue, Option<&'a str>)>,
{
    let mut blocks = Vec::new();
    for (idx, (cue, translated_opt)) in cues.into_iter().enumerate() {
        let seq = cue
            .meta
            .srt_sequence
            .clone()
            .unwrap_or_else(|| (idx + 1).to_string());
        let body = match cue.cue_type {
            CueType::Dialogue => translated_opt.unwrap_or(&cue.original_text),
            CueType::Comment | CueType::Drawing => &cue.original_text,
        };
        if body.is_empty() {
            blocks.push(format!("{seq}\n{} --> {}", cue.start_time, cue.end_time));
        } else {
            blocks.push(format!(
                "{seq}\n{} --> {}\n{body}",
                cue.start_time, cue.end_time
            ));
        }
    }
    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

fn parse_timing_line(line: &str, cue_index: i64) -> AppResult<(String, String)> {
    let Some((left, right)) = line.split_once("-->") else {
        return Err(AppError::InvalidInput(format!(
            "Invalid timestamp in SRT cue #{cue_index}."
        )));
    };
    let start = left.trim();
    let end = right.trim();
    if start.is_empty() || end.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "Incomplete timestamp in SRT cue #{cue_index}."
        )));
    }
    Ok((start.to_string(), end.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_srt_with_multiline_dialogue_and_tags() {
        let sample = "\
1
00:00:01,000 --> 00:00:03,250
<i>Hello there,</i>
General Kenobi!

2
00:00:04,000 --> 00:00:06,500 X1:40 X2:600
Move fast!
";
        let doc = parse_srt(sample, "UTF-8".to_string(), false).expect("parse srt");
        assert_eq!(doc.cues.len(), 2);
        assert_eq!(doc.cues[0].cue_type, CueType::Dialogue);
        assert_eq!(
            doc.cues[0].original_text,
            "<i>Hello there,</i>\nGeneral Kenobi!"
        );
        assert_eq!(doc.cues[1].end_time, "00:00:06,500 X1:40 X2:600");

        let serialized = serialize_srt(doc.cues.iter().map(|c| (c, None)));
        assert_eq!(serialized, sample);
    }
}
