use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use thiserror::Error;

use super::types::{InlineTagKind, InlineTagMapping, TagShieldResult};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TagRestoreError {
    #[error("Missing inline tag placeholder '{0}' in model output.")]
    MissingPlaceholder(String),
    #[error("Duplicate inline tag placeholder '{0}' in model output.")]
    DuplicatePlaceholder(String),
    #[error("Mismatched order for inline tag pair '<t{0}>...</t{0}>'.")]
    MismatchedOrder(usize),
    #[error("Unexpected foreign tag placeholder remaining in model output.")]
    UnexpectedPlaceholder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    AssBlock,
    AssEscape,
    HtmlOpen(String),
    HtmlClose(String),
    SelfClosingTag,
}

#[derive(Debug, Clone)]
struct TagToken {
    start: usize,
    end: usize,
    raw: String,
    kind: TokenKind,
}

fn tag_token_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \{[^}\n]*\}
            | \\[Nnh]
            | <br\s*/?>
            | <\d{1,2}:\d{2}(?::\d{2})?\.\d{3}>
            | </(?:i|b|u|s|font|ruby|rt|rp|c|v|lang)\b[^>]*>
            | <(?:i|b|u|s|font|ruby|rt|rp|c|v|lang)(?:[\s.][^>]*)?>
            ",
        )
        .expect("valid tag regex")
    })
}

fn drawing_mode_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\\p[1-9]\d*(?:[^a-zA-Z0-9]|$)").expect("valid ass drawing regex")
    })
}

fn leftover_placeholder_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"</?t\d+\s*/?>").expect("valid placeholder regex"))
}

/// Checks whether an ASS dialogue text enables vector drawing commands (`\p1`..`\p9`).
pub fn contains_ass_drawing_command(text: &str) -> bool {
    let re = drawing_mode_regex();
    for m in tag_token_regex().find_iter(text) {
        let slice = m.as_str();
        if slice.starts_with('{') && re.is_match(slice) {
            return true;
        }
    }
    false
}

/// Extracts compact tag labels for UI badge pills (deduplicated, max 8 badges).
pub fn extract_display_tags(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for m in tag_token_regex().find_iter(text) {
        let raw = m.as_str().trim();
        if raw.is_empty() {
            continue;
        }
        let label = compact_display_label(raw);
        if seen.insert(label.clone()) {
            out.push(label);
            if out.len() >= 8 {
                break;
            }
        }
    }
    out
}

fn compact_display_label(raw: &str) -> String {
    if raw.chars().count() <= 20 {
        raw.to_string()
    } else {
        let prefix: String = raw.chars().take(17).collect();
        format!("{prefix}...")
    }
}

/// Shields formatting tags deterministically before LLM translation.
pub fn shield_tags(original_text: &str) -> TagShieldResult {
    let display_tags = extract_display_tags(original_text);
    if contains_ass_drawing_command(original_text) {
        return TagShieldResult {
            clean_text: String::new(),
            leading_tags: original_text.to_string(),
            trailing_tags: String::new(),
            inline_mappings: Vec::new(),
            display_tags,
            is_drawing: true,
        };
    }

    let (leading_tags, core_text, trailing_tags) = peel_outer_tags(original_text);
    if core_text.is_empty() {
        return TagShieldResult {
            clean_text: String::new(),
            leading_tags,
            trailing_tags,
            inline_mappings: Vec::new(),
            display_tags,
            is_drawing: false,
        };
    }

    let tokens = tokenize_tags(&core_text);
    if tokens.is_empty() {
        return TagShieldResult {
            clean_text: core_text,
            leading_tags,
            trailing_tags,
            inline_mappings: Vec::new(),
            display_tags,
            is_drawing: false,
        };
    }

    // Match paired inline tags via stack so each pair shares one placeholder ID.
    let mut paired_close_for = vec![None; tokens.len()];
    let mut is_matched_close = vec![false; tokens.len()];
    let mut html_stack: Vec<(String, usize)> = Vec::new();
    let mut ass_italic_stack: Vec<usize> = Vec::new();
    let mut ass_bold_stack: Vec<usize> = Vec::new();
    let mut ass_underline_stack: Vec<usize> = Vec::new();

    for (idx, tok) in tokens.iter().enumerate() {
        match &tok.kind {
            TokenKind::HtmlOpen(name) => {
                html_stack.push((name.clone(), idx));
            }
            TokenKind::HtmlClose(name) => {
                if let Some(pos) = html_stack.iter().rposition(|(n, _)| n == name) {
                    let (_, open_idx) = html_stack.remove(pos);
                    paired_close_for[open_idx] = Some(idx);
                    is_matched_close[idx] = true;
                }
            }
            TokenKind::AssBlock => {
                if tok.raw == "{\\i1}" {
                    ass_italic_stack.push(idx);
                } else if tok.raw == "{\\i0}" {
                    if let Some(open_idx) = ass_italic_stack.pop() {
                        paired_close_for[open_idx] = Some(idx);
                        is_matched_close[idx] = true;
                    }
                } else if tok.raw == "{\\b1}" {
                    ass_bold_stack.push(idx);
                } else if tok.raw == "{\\b0}" {
                    if let Some(open_idx) = ass_bold_stack.pop() {
                        paired_close_for[open_idx] = Some(idx);
                        is_matched_close[idx] = true;
                    }
                } else if tok.raw == "{\\u1}" {
                    ass_underline_stack.push(idx);
                } else if tok.raw == "{\\u0}" {
                    if let Some(open_idx) = ass_underline_stack.pop() {
                        paired_close_for[open_idx] = Some(idx);
                        is_matched_close[idx] = true;
                    }
                }
            }
            TokenKind::AssEscape | TokenKind::SelfClosingTag => {}
        }
    }

    let mut next_id = 1usize;
    let mut open_id_for_token = vec![None; tokens.len()];
    let mut close_id_for_token = vec![None; tokens.len()];
    let mut inline_mappings = Vec::new();

    for idx in 0..tokens.len() {
        if is_matched_close[idx] {
            continue;
        }
        let id = next_id;
        next_id += 1;
        if let Some(close_idx) = paired_close_for[idx] {
            open_id_for_token[idx] = Some(id);
            close_id_for_token[close_idx] = Some(id);
            inline_mappings.push(InlineTagMapping {
                id,
                kind: InlineTagKind::Paired,
                open_tag: tokens[idx].raw.clone(),
                close_tag: Some(tokens[close_idx].raw.clone()),
            });
        } else {
            open_id_for_token[idx] = Some(id);
            inline_mappings.push(InlineTagMapping {
                id,
                kind: InlineTagKind::SelfClosing,
                open_tag: tokens[idx].raw.clone(),
                close_tag: None,
            });
        }
    }

    let mut clean_text = String::with_capacity(core_text.len() + tokens.len() * 6);
    let mut cursor = 0usize;

    for (idx, tok) in tokens.iter().enumerate() {
        if tok.start > cursor {
            clean_text.push_str(&core_text[cursor..tok.start]);
        }
        if let Some(id) = close_id_for_token[idx] {
            clean_text.push_str(&format!("</t{id}>"));
        } else if let Some(id) = open_id_for_token[idx] {
            if paired_close_for[idx].is_some() {
                clean_text.push_str(&format!("<t{id}>"));
            } else {
                clean_text.push_str(&format!("<t{id}/>"));
            }
        }
        cursor = tok.end;
    }
    if cursor < core_text.len() {
        clean_text.push_str(&core_text[cursor..]);
    }

    TagShieldResult {
        clean_text,
        leading_tags,
        trailing_tags,
        inline_mappings,
        display_tags,
        is_drawing: false,
    }
}

/// Restores all leading, inline, and trailing tags onto the translated clean string.
///
/// Rejects corrupted outputs where placeholders were dropped, duplicated, reordered out of pair,
/// or invented by the LLM so the caller can invoke Bounded Self-Healing.
pub fn restore_tags(
    shield: &TagShieldResult,
    translated_clean: &str,
) -> Result<String, TagRestoreError> {
    if shield.is_drawing {
        return Ok(shield.leading_tags.clone());
    }

    let mut work = translated_clean.to_string();

    for mapping in &shield.inline_mappings {
        match mapping.kind {
            InlineTagKind::Paired => {
                let open_ph = format!("<t{}>", mapping.id);
                let close_ph = format!("</t{}>", mapping.id);

                let open_count = work.matches(&open_ph).count();
                if open_count == 0 {
                    return Err(TagRestoreError::MissingPlaceholder(open_ph));
                }
                if open_count > 1 {
                    return Err(TagRestoreError::DuplicatePlaceholder(open_ph));
                }

                let close_count = work.matches(&close_ph).count();
                if close_count == 0 {
                    return Err(TagRestoreError::MissingPlaceholder(close_ph));
                }
                if close_count > 1 {
                    return Err(TagRestoreError::DuplicatePlaceholder(close_ph));
                }

                let open_pos = work.find(&open_ph).expect("checked open_count == 1");
                let close_pos = work.find(&close_ph).expect("checked close_count == 1");
                if open_pos > close_pos {
                    return Err(TagRestoreError::MismatchedOrder(mapping.id));
                }

                work = work.replacen(&open_ph, &mapping.open_tag, 1);
                work = work.replacen(
                    &close_ph,
                    mapping.close_tag.as_deref().unwrap_or(""),
                    1,
                );
            }
            InlineTagKind::SelfClosing => {
                let ph = format!("<t{}/>", mapping.id);
                let spaced_ph = format!("<t{} />", mapping.id);

                let count = work.matches(&ph).count() + work.matches(&spaced_ph).count();
                if count == 0 {
                    return Err(TagRestoreError::MissingPlaceholder(ph));
                }
                if count > 1 {
                    return Err(TagRestoreError::DuplicatePlaceholder(ph));
                }

                if work.contains(&ph) {
                    work = work.replacen(&ph, &mapping.open_tag, 1);
                } else {
                    work = work.replacen(&spaced_ph, &mapping.open_tag, 1);
                }
            }
        }
    }

    if leftover_placeholder_regex().is_match(&work) {
        return Err(TagRestoreError::UnexpectedPlaceholder);
    }

    let mut out = String::with_capacity(
        shield.leading_tags.len() + work.len() + shield.trailing_tags.len(),
    );
    out.push_str(&shield.leading_tags);
    out.push_str(&work);
    out.push_str(&shield.trailing_tags);
    Ok(out)
}

/// Peels leading/trailing ASS override blocks and balanced whole-line wrappers without
/// breaking partial inline pairs (e.g., never strips `<i>` from `<i>a</i> and <b>b</b>`).
fn peel_outer_tags(input: &str) -> (String, String, String) {
    let mut leading = String::new();
    let mut trailing = String::new();
    let mut s = input;

    loop {
        let mut changed = false;

        // 1. Peel leading ASS override blocks unless they are balanced open pairs like `{\i1}...`
        // that close mid-line. Actually `{\pos(...)}`, `{\an8}`, etc. never act as scoped end-tags.
        while let Some(first_tok) = leading_ass_nonpaired_tag(s) {
            leading.push_str(&s[..first_tok.len()]);
            s = &s[first_tok.len()..];
            changed = true;
        }

        // 2. Peel trailing ASS reset/override blocks at the very end if no plain text follows.
        while let Some(last_len) = trailing_ass_nonpaired_tag_len(s) {
            let split_at = s.len() - last_len;
            trailing = format!("{}{}", &s[split_at..], trailing);
            s = &s[..split_at];
            changed = true;
        }

        // 3. Peel balanced outer wrappers spanning the ENTIRE remaining string
        //    (e.g., `<i>...</i>`, `<font color="...">...</font>`, `{\i1}...{\i0}`).
        if let Some((open, inner, close)) = strip_outer_balanced_wrapper(s) {
            leading.push_str(open);
            trailing = format!("{close}{trailing}");
            s = inner;
            changed = true;
        }

        if !changed {
            break;
        }
    }

    (leading, s.to_string(), trailing)
}

fn leading_ass_nonpaired_tag(s: &str) -> Option<&str> {
    let m = tag_token_regex().find(s)?;
    if m.start() != 0 {
        return None;
    }
    let slice = m.as_str();
    if slice.starts_with('{') && !matches!(slice, "{\\i1}" | "{\\b1}" | "{\\u1}") {
        return Some(slice);
    }
    None
}

fn trailing_ass_nonpaired_tag_len(s: &str) -> Option<usize> {
    let mut last_match = None;
    for m in tag_token_regex().find_iter(s) {
        if m.end() == s.len() {
            last_match = Some(m);
        }
    }
    let m = last_match?;
    let slice = m.as_str();
    if slice.starts_with('{') && !matches!(slice, "{\\i0}" | "{\\b0}" | "{\\u0}") {
        return Some(slice.len());
    }
    None
}

fn strip_outer_balanced_wrapper(s: &str) -> Option<(&str, &str, &str)> {
    let tokens = tokenize_tags(s);
    if tokens.len() < 2 {
        return None;
    }
    let first = tokens.first()?;
    let last = tokens.last()?;
    if first.start != 0 || last.end != s.len() {
        return None;
    }

    // Check ASS whole-line pairs
    let ass_pair = matches!(
        (first.raw.as_str(), last.raw.as_str()),
        ("{\\i1}", "{\\i0}")
            | ("{\\i1}", "{\\r}")
            | ("{\\b1}", "{\\b0}")
            | ("{\\u1}", "{\\u0}")
    );
    if ass_pair && !tokens[1..tokens.len() - 1]
        .iter()
        .any(|t| t.raw == last.raw)
    {
        return Some((&s[..first.end], &s[first.end..last.start], &s[last.start..]));
    }

    // Check HTML/VTT whole-line wrapper: opening tag at 0 must stay unclosed until the final token.
    if let (TokenKind::HtmlOpen(open_name), TokenKind::HtmlClose(close_name)) =
        (&first.kind, &last.kind)
    {
        if open_name != close_name {
            return None;
        }
        let mut depth = 1usize;
        for tok in &tokens[1..tokens.len() - 1] {
            match &tok.kind {
                TokenKind::HtmlOpen(n) if n == open_name => depth += 1,
                TokenKind::HtmlClose(n) if n == open_name => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        // Closed earlier in the middle, so not a whole-line wrapper!
                        return None;
                    }
                }
                _ => {}
            }
        }
        if depth == 1 {
            return Some((&s[..first.end], &s[first.end..last.start], &s[last.start..]));
        }
    }

    None
}

fn tokenize_tags(text: &str) -> Vec<TagToken> {
    tag_token_regex()
        .find_iter(text)
        .map(|m| {
            let raw = m.as_str().to_string();
            let kind = classify_token(&raw);
            TagToken {
                start: m.start(),
                end: m.end(),
                raw,
                kind,
            }
        })
        .collect()
}

fn classify_token(raw: &str) -> TokenKind {
    if raw.starts_with('{') {
        return TokenKind::AssBlock;
    }
    if raw.starts_with('\\') {
        return TokenKind::AssEscape;
    }
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with("<br") || raw.as_bytes().get(1).is_some_and(u8::is_ascii_digit) {
        return TokenKind::SelfClosingTag;
    }
    if let Some(rest) = lower.strip_prefix("</") {
        let name = rest
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()
            .unwrap_or_default()
            .to_string();
        return TokenKind::HtmlClose(name);
    }
    if let Some(rest) = lower.strip_prefix('<') {
        let name = rest
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()
            .unwrap_or_default()
            .to_string();
        return TokenKind::HtmlOpen(name);
    }
    TokenKind::SelfClosingTag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ass_drawing_commands_accurately() {
        assert!(contains_ass_drawing_command(
            "{\\an7\\pos(100,200)\\p1}m 0 0 l 100 0 100 100 0 100"
        ));
        assert!(!contains_ass_drawing_command(
            "{\\pos(320,240)\\p0}Normal dialogue line"
        ));
        assert!(!contains_ass_drawing_command("x < 5 and y > 2"));
    }

    #[test]
    fn shields_and_restores_leading_inline_and_newline_tags() {
        let input = "{\\an8\\pos(640,50)}<i>Watch out for <b>that</b>\\Ncar!</i>";
        let shield = shield_tags(input);

        assert!(!shield.is_drawing);
        assert_eq!(shield.leading_tags, "{\\an8\\pos(640,50)}<i>");
        assert_eq!(shield.trailing_tags, "</i>");
        assert_eq!(shield.clean_text, "Watch out for <t1>that</t1><t2/>car!");

        let restored = restore_tags(&shield, "Awas dengan <t1>mobil</t1><t2/>itu!")
            .expect("restore tags should succeed");
        assert_eq!(
            restored,
            "{\\an8\\pos(640,50)}<i>Awas dengan <b>mobil</b>\\Nitu!</i>"
        );
    }

    #[test]
    fn does_not_split_partial_outer_wrappers() {
        let input = "<i>Hello</i> and <b>world</b>";
        let shield = shield_tags(input);

        assert_eq!(shield.leading_tags, "");
        assert_eq!(shield.trailing_tags, "");
        assert_eq!(shield.clean_text, "<t1>Hello</t1> and <t2>world</t2>");

        let restored = restore_tags(&shield, "<t1>Halo</t1> dan <t2>dunia</t2>").unwrap();
        assert_eq!(restored, "<i>Halo</i> dan <b>dunia</b>");
    }

    #[test]
    fn rejects_dropped_or_corrupted_placeholders() {
        let shield = shield_tags("Line one\\NLine two <i>fast</i>");
        assert!(restore_tags(&shield, "Baris satu<t1/>Baris dua cepat").is_err());
        assert!(restore_tags(&shield, "Baris satu<t1/>Baris dua </t2>cepat<t2>").is_err());
        assert!(restore_tags(&shield, "Baris satu<t1/>Baris dua <t2>cepat</t2> <t9/>").is_err());
    }
}
