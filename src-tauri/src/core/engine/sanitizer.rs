const OPEN_TAG: &str = "<think>";
const CLOSE_TAG: &str = "</think>";

/// Strips all `<think>...</think>` reasoning blocks (including unclosed trailing `<think>...`)
/// from a complete model output string.
pub fn strip_think_tags(input: &str) -> String {
    let mut sanitizer = StreamingThinkSanitizer::new();
    let mut out = sanitizer.push(input);
    out.push_str(&sanitizer.finish());
    out
}

/// Incremental `<think>...</think>` sanitizer for SSE token streams where tag boundaries
/// may be split across multiple token chunks.
#[derive(Debug, Default)]
pub struct StreamingThinkSanitizer {
    in_think: bool,
    pending: String,
    trim_leading_ws: bool,
}

impl StreamingThinkSanitizer {
    pub fn new() -> Self {
        Self {
            in_think: false,
            pending: String::new(),
            trim_leading_ws: false,
        }
    }

    pub fn push(&mut self, chunk: &str) -> String {
        if chunk.is_empty() {
            return String::new();
        }
        self.pending.push_str(chunk);
        let mut out = String::new();

        loop {
            if self.in_think {
                if let Some(idx) = self.pending.find(CLOSE_TAG) {
                    let after_idx = idx + CLOSE_TAG.len();
                    let remainder = self.pending[after_idx..].to_string();
                    self.pending = remainder;
                    self.in_think = false;
                    self.trim_leading_ws = true;
                } else {
                    let keep = trailing_partial_prefix_len(&self.pending, CLOSE_TAG);
                    let drain_len = self.pending.len().saturating_sub(keep);
                    if drain_len > 0 {
                        self.pending.drain(..drain_len);
                    }
                    break;
                }
            } else if let Some(idx) = self.pending.find(OPEN_TAG) {
                let before = self.pending[..idx].to_string();
                self.emit_text(&before, &mut out);
                let after_idx = idx + OPEN_TAG.len();
                self.pending = self.pending[after_idx..].to_string();
                self.in_think = true;
            } else {
                let keep = trailing_partial_prefix_len(&self.pending, OPEN_TAG);
                let safe_len = self.pending.len().saturating_sub(keep);
                if safe_len > 0 {
                    let safe_slice = self.pending[..safe_len].to_string();
                    self.emit_text(&safe_slice, &mut out);
                    self.pending.drain(..safe_len);
                }
                break;
            }
        }

        out
    }

    pub fn finish(&mut self) -> String {
        let mut out = String::new();
        if !self.in_think && !self.pending.is_empty() {
            let rem = std::mem::take(&mut self.pending);
            self.emit_text(&rem, &mut out);
        } else {
            self.pending.clear();
        }
        self.in_think = false;
        self.trim_leading_ws = false;
        out
    }

    fn emit_text(&mut self, text: &str, out: &mut String) {
        if text.is_empty() {
            return;
        }
        if self.trim_leading_ws {
            let trimmed = text.trim_start_matches(['\r', '\n', ' ', '\t']);
            if trimmed.is_empty() {
                return;
            }
            self.trim_leading_ws = false;
            out.push_str(trimmed);
        } else {
            out.push_str(text);
        }
    }
}

fn trailing_partial_prefix_len(text: &str, tag: &str) -> usize {
    let max_len = (tag.len() - 1).min(text.len());
    for len in (1..=max_len).rev() {
        if text.is_char_boundary(text.len() - len) && tag.starts_with(&text[text.len() - len..]) {
            return len;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_complete_and_unclosed_think_blocks() {
        assert_eq!(
            strip_think_tags("<think>reasoning here</think>\n\nHalo dunia."),
            "Halo dunia."
        );
        assert_eq!(
            strip_think_tags("Awal <think>hidden</think>akhir"),
            "Awal akhir"
        );
        assert_eq!(
            strip_think_tags("Bagian valid<think>terpotong tanpa tutup"),
            "Bagian valid"
        );
    }

    #[test]
    fn streaming_sanitizer_handles_split_tag_boundaries() {
        let chunks = [
            "<th",
            "ink>rahasia",
            " internal</th",
            "ink>\n\n  Selamat ",
            "pagi",
            " <thi",
            "nk>skip</",
            "think>semua!",
        ];
        let mut sanitizer = StreamingThinkSanitizer::new();
        let mut collected = String::new();
        for c in chunks {
            collected.push_str(&sanitizer.push(c));
        }
        collected.push_str(&sanitizer.finish());
        assert_eq!(collected, "Selamat pagi semua!");
    }

    #[test]
    fn streaming_sanitizer_preserves_normal_angle_brackets() {
        let chunks = ["<t1>Teks ", "miring</t1> < 5"];
        let mut sanitizer = StreamingThinkSanitizer::new();
        let mut collected = String::new();
        for c in chunks {
            collected.push_str(&sanitizer.push(c));
        }
        collected.push_str(&sanitizer.finish());
        assert_eq!(collected, "<t1>Teks miring</t1> < 5");
    }
}
