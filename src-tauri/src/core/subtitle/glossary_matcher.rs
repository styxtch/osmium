use std::collections::HashSet;

use regex::RegexBuilder;

use super::types::GlossaryMatch;
use crate::db::models::GlossaryEntry;

/// Finds glossary entries whose `source_term` appears as a standalone word/phrase
/// inside `clean_text` (Unicode word-boundary aware).
pub fn match_glossary_terms(clean_text: &str, entries: &[GlossaryEntry]) -> Vec<GlossaryMatch> {
    if clean_text.is_empty() || entries.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let mut seen_keys = HashSet::new();

    // Sort longest terms first so compound terms match deterministically alongside shorter ones.
    let mut ordered: Vec<&GlossaryEntry> = entries.iter().collect();
    ordered.sort_by(|a, b| b.source_term.len().cmp(&a.source_term.len()));

    for entry in ordered {
        let term = entry.source_term.trim();
        if term.is_empty() {
            continue;
        }
        let target = entry.target_term.trim();
        if target.is_empty() {
            continue;
        }

        if contains_bounded_term(clean_text, term, entry.case_sensitive) {
            let dedup_key = if entry.case_sensitive {
                term.to_string()
            } else {
                term.to_lowercase()
            };
            if seen_keys.insert(dedup_key) {
                matches.push(GlossaryMatch {
                    source_term: term.to_string(),
                    target_term: target.to_string(),
                    case_sensitive: entry.case_sensitive,
                });
            }
        }
    }

    matches
}

fn contains_bounded_term(haystack: &str, term: &str, case_sensitive: bool) -> bool {
    let escaped = regex::escape(term);
    let Ok(re) = RegexBuilder::new(&escaped)
        .case_insensitive(!case_sensitive)
        .build()
    else {
        return false;
    };

    let first_is_word = term.chars().next().is_some_and(is_word_char);
    let last_is_word = term.chars().next_back().is_some_and(is_word_char);

    for m in re.find_iter(haystack) {
        let left_ok = if first_is_word {
            haystack[..m.start()]
                .chars()
                .next_back()
                .is_none_or(|c| !is_word_char(c))
        } else {
            true
        };

        let right_ok = if last_is_word {
            haystack[m.end()..]
                .chars()
                .next()
                .is_none_or(|c| !is_word_char(c))
        } else {
            true
        };

        if left_ok && right_ok {
            return true;
        }
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(src: &str, dst: &str, case_sensitive: bool) -> GlossaryEntry {
        GlossaryEntry {
            id: format!("e-{src}"),
            glossary_id: "g-1".to_string(),
            source_term: src.to_string(),
            target_term: dst.to_string(),
            case_sensitive,
        }
    }

    #[test]
    fn matches_unicode_word_boundaries_and_respects_case_flag() {
        let entries = vec![
            make_entry("mage", "penyihir", false),
            make_entry("NERV", "NERV", true),
            make_entry("AT Field", "AT Field", false),
        ];

        // "damage" and "nervous" must NOT match "mage" or "NERV"
        let non_match = match_glossary_terms(
            "Massive damage to nervous system and at fields.",
            &entries,
        );
        assert!(non_match.is_empty());

        let hit = match_glossary_terms(
            "Mage, raise the AT field against NERV units!",
            &entries,
        );
        assert_eq!(hit.len(), 3);
        assert!(hit.iter().any(|m| m.source_term == "mage"));
        assert!(hit.iter().any(|m| m.source_term == "NERV"));
        assert!(hit.iter().any(|m| m.source_term == "AT Field"));
    }
}
