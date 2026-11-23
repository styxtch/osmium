use crate::error::{AppError, AppResult};

pub const MAX_TEXT_INPUT_CHARS: usize = 25_000;

pub fn chunk_threshold_for_ctx(ctx_size: u32) -> usize {
    if ctx_size <= 2048 {
        2_500
    } else {
        5_000
    }
}

/// Validates input length (`<= 25,000` chars) and splits long text at paragraph/sentence
/// boundaries so each chunk fits comfortably within the active context window.
pub fn split_into_chunks(text: &str, max_chunk_chars: usize) -> AppResult<Vec<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            "Source text cannot be empty".to_string(),
        ));
    }

    let total_chars = text.chars().count();
    if total_chars > MAX_TEXT_INPUT_CHARS {
        return Err(AppError::InvalidInput(format!(
            "Text exceeds the {MAX_TEXT_INPUT_CHARS}-character safety limit ({total_chars} characters)."
        )));
    }

    let limit = max_chunk_chars.max(500);
    if total_chars <= limit {
        return Ok(vec![text.to_string()]);
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;

    for para in text.split("\n\n") {
        let para_chars = para.chars().count();
        if para_chars > limit {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
                current_chars = 0;
            }
            for sub in split_oversized_paragraph(para, limit) {
                chunks.push(sub);
            }
            continue;
        }

        let sep_chars = if current.is_empty() { 0 } else { 2 };
        if current_chars + sep_chars + para_chars <= limit {
            if !current.is_empty() {
                current.push_str("\n\n");
                current_chars += 2;
            }
            current.push_str(para);
            current_chars += para_chars;
        } else {
            if !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            current = para.to_string();
            current_chars = para_chars;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    Ok(chunks)
}

fn split_oversized_paragraph(para: &str, limit: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut remaining = para.trim();

    while !remaining.is_empty() {
        if remaining.chars().count() <= limit {
            parts.push(remaining.to_string());
            break;
        }

        let byte_limit = char_idx_byte(remaining, limit);
        let window = &remaining[..byte_limit];

        let split_at = window
            .rfind('\n')
            .or_else(|| window.rfind(". ").map(|i| i + 1))
            .or_else(|| window.rfind("! ").map(|i| i + 1))
            .or_else(|| window.rfind("? ").map(|i| i + 1))
            .or_else(|| window.rfind(' '))
            .filter(|&i| i > 0)
            .unwrap_or(byte_limit);

        let piece = remaining[..split_at].trim();
        if !piece.is_empty() {
            parts.push(piece.to_string());
        }
        remaining = remaining[split_at..].trim_start();
    }

    parts
}

fn char_idx_byte(s: &str, char_limit: usize) -> usize {
    s.char_indices()
        .nth(char_limit)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_or_above_25000_chars() {
        assert!(split_into_chunks("   ", 5000).is_err());
        let over = "a".repeat(25_001);
        assert!(split_into_chunks(&over, 5000).is_err());
        let exact = "a".repeat(25_000);
        assert!(split_into_chunks(&exact, 5000).is_ok());
    }

    #[test]
    fn ctx_threshold_adjusts_for_cpu_2048_vs_gpu() {
        assert_eq!(chunk_threshold_for_ctx(2048), 2500);
        assert_eq!(chunk_threshold_for_ctx(4096), 5000);
        assert_eq!(chunk_threshold_for_ctx(8192), 5000);
    }

    #[test]
    fn splits_multi_paragraph_and_oversized_text_cleanly() {
        let p1 = "Sentence one. ".repeat(200); // ~2800 chars
        let p2 = "Sentence two. ".repeat(200); // ~2800 chars
        let combined = format!("{p1}\n\n{p2}");

        let chunks = split_into_chunks(&combined, 5000).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(chunks.iter().all(|c| c.chars().count() <= 5000));

        let single_huge = "Hello world. ".repeat(450); // ~5850 chars
        let split_single = split_into_chunks(&single_huge, 2500).unwrap();
        assert!(split_single.len() >= 3);
        assert!(split_single.iter().all(|c| c.chars().count() <= 2500));
    }
}
