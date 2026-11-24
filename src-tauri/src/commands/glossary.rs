use crate::core::subtitle::exporter::write_atomic_export;
use crate::db::models::{Glossary, GlossaryEntry};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

const MAX_GLOSSARY_FILE_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryImportResult {
    pub glossary: Glossary,
    pub imported_count: usize,
    pub skipped_count: usize,
}

#[derive(Debug, Serialize)]
struct ExportGlossaryEntry<'a> {
    source_term: &'a str,
    target_term: &'a str,
    case_sensitive: bool,
}

#[derive(Debug, Serialize)]
struct ExportGlossaryPayload<'a> {
    name: &'a str,
    entries: Vec<ExportGlossaryEntry<'a>>,
}

fn serialize_glossary_json(name: &str, entries: &[GlossaryEntry]) -> AppResult<String> {
    let payload = ExportGlossaryPayload {
        name,
        entries: entries
            .iter()
            .map(|e| ExportGlossaryEntry {
                source_term: &e.source_term,
                target_term: &e.target_term,
                case_sensitive: e.case_sensitive,
            })
            .collect(),
    };
    serde_json::to_string_pretty(&payload)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize glossary JSON: {e}")))
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

fn serialize_glossary_csv(entries: &[GlossaryEntry]) -> String {
    let mut out = String::from("source_term,target_term,case_sensitive\r\n");
    for entry in entries {
        out.push_str(&csv_escape(&entry.source_term));
        out.push(',');
        out.push_str(&csv_escape(&entry.target_term));
        out.push(',');
        out.push_str(if entry.case_sensitive { "true" } else { "false" });
        out.push_str("\r\n");
    }
    out
}

fn extract_str_field(obj: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> String {
    for key in keys {
        if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
            return val.to_string();
        }
    }
    String::new()
}

fn parse_glossary_json(raw: &str) -> AppResult<(Option<String>, Vec<(String, String, bool)>)> {
    let clean = raw.trim_start_matches('\u{FEFF}').trim();
    let parsed: serde_json::Value = serde_json::from_str(clean)
        .map_err(|e| AppError::InvalidInput(format!("Invalid glossary JSON: {e}")))?;

    let (name, arr) = match &parsed {
        serde_json::Value::Array(items) => (None, items),
        serde_json::Value::Object(map) => {
            let name = map
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let entries = map
                .get("entries")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    AppError::InvalidInput(
                        "Glossary JSON object must contain an 'entries' array".to_string(),
                    )
                })?;
            (name, entries)
        }
        _ => {
            return Err(AppError::InvalidInput(
                "Glossary JSON must be an array or an object with 'entries'".to_string(),
            ));
        }
    };

    let mut rows = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(obj) = item.as_object() else {
            rows.push((String::new(), String::new(), false));
            continue;
        };
        let src = extract_str_field(obj, &["source_term", "sourceTerm", "source"]);
        let tgt = extract_str_field(obj, &["target_term", "targetTerm", "target"]);
        let cs = obj
            .get("case_sensitive")
            .or_else(|| obj.get("caseSensitive"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        rows.push((src, tgt, cs));
    }

    Ok((name, rows))
}

fn parse_csv_records(raw: &str) -> Vec<Vec<String>> {
    let clean = raw.trim_start_matches('\u{FEFF}');
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = clean.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(ch);
            }
        } else {
            match ch {
                '"' if field.is_empty() => {
                    in_quotes = true;
                }
                ',' => {
                    row.push(std::mem::take(&mut field));
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    row.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut row));
                }
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut row));
                }
                _ => {
                    field.push(ch);
                }
            }
        }
    }

    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        records.push(row);
    }

    records
}

fn parse_glossary_csv(raw: &str) -> Vec<(String, String, bool)> {
    let records = parse_csv_records(raw);
    let mut iter = records.into_iter().peekable();

    if let Some(first) = iter.peek() {
        let c0 = first.first().map(|s| s.trim().to_ascii_lowercase()).unwrap_or_default();
        let c1 = first.get(1).map(|s| s.trim().to_ascii_lowercase()).unwrap_or_default();
        let looks_like_header = matches!(c0.as_str(), "source_term" | "sourceterm" | "source" | "term")
            && matches!(
                c1.as_str(),
                "target_term" | "targetterm" | "target" | "translation"
            );
        if looks_like_header {
            iter.next();
        }
    }

    let mut out = Vec::new();
    for row in iter {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        if row.len() < 2 {
            out.push((String::new(), String::new(), false));
            continue;
        }
        let cs = row
            .get(2)
            .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);
        out.push((row[0].clone(), row[1].clone(), cs));
    }
    out
}

#[tauri::command]
pub async fn list_glossaries(state: State<'_, AppState>) -> AppResult<Vec<Glossary>> {
    state.db.list_glossaries()
}

#[tauri::command]
pub async fn create_glossary(state: State<'_, AppState>, name: String) -> AppResult<Glossary> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            "Glossary collection name cannot be empty".to_string(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    state.db.create_glossary(&id, trimmed)
}

#[tauri::command]
pub async fn update_glossary(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> AppResult<Glossary> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput(
            "Glossary collection name cannot be empty".to_string(),
        ));
    }
    state.db.update_glossary_name(&id, trimmed)
}

#[tauri::command]
pub async fn delete_glossary(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.db.delete_glossary(&id)
}

#[tauri::command]
pub async fn list_glossary_entries(
    state: State<'_, AppState>,
    glossary_id: String,
) -> AppResult<Vec<GlossaryEntry>> {
    state.db.list_glossary_entries(&glossary_id)
}

#[tauri::command]
pub async fn upsert_glossary_entry(
    state: State<'_, AppState>,
    id: Option<String>,
    glossary_id: String,
    source_term: String,
    target_term: String,
    case_sensitive: bool,
) -> AppResult<GlossaryEntry> {
    let src = source_term.trim().to_string();
    let tgt = target_term.trim().to_string();
    if src.is_empty() || tgt.is_empty() {
        return Err(AppError::InvalidInput(
            "Source and target terms cannot be empty".to_string(),
        ));
    }

    let entry = GlossaryEntry {
        id: id
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        glossary_id,
        source_term: src,
        target_term: tgt,
        case_sensitive,
    };

    state.db.upsert_glossary_entry(&entry).map_err(|err| match err {
        AppError::DbError(msg) if msg.contains("UNIQUE constraint") => AppError::InvalidInput(
            "A source term with the same case-sensitivity setting already exists in this glossary"
                .to_string(),
        ),
        other => other,
    })?;

    Ok(entry)
}

#[tauri::command]
pub async fn delete_glossary_entry(
    state: State<'_, AppState>,
    id: String,
    glossary_id: String,
) -> AppResult<()> {
    state.db.delete_glossary_entry(&id, &glossary_id)
}

#[tauri::command]
pub async fn export_glossary(
    state: State<'_, AppState>,
    glossary_id: String,
    dest_path: String,
    format: Option<String>,
) -> AppResult<String> {
    let glossary = state
        .db
        .list_glossaries()?
        .into_iter()
        .find(|g| g.id == glossary_id)
        .ok_or_else(|| AppError::NotFound(format!("Glossary '{glossary_id}'")))?;
    let entries = state.db.list_glossary_entries(&glossary_id)?;

    let use_csv = format
        .as_deref()
        .map(|f| f.trim().eq_ignore_ascii_case("csv"))
        .unwrap_or_else(|| dest_path.trim().to_ascii_lowercase().ends_with(".csv"));

    let (content, include_bom) = if use_csv {
        (serialize_glossary_csv(&entries), true)
    } else {
        (serialize_glossary_json(&glossary.name, &entries)?, false)
    };

    write_atomic_export(Path::new(&dest_path), &content, include_bom)?;
    tracing::info!(
        format = if use_csv { "csv" } else { "json" },
        entry_count = entries.len(),
        "Exported glossary file"
    );
    Ok(dest_path)
}

#[tauri::command]
pub async fn import_glossary(
    state: State<'_, AppState>,
    file_path: String,
    target_glossary_id: Option<String>,
    collection_name: Option<String>,
) -> AppResult<GlossaryImportResult> {
    let path = Path::new(&file_path);
    if !path.is_file() {
        return Err(AppError::NotFound(format!("Glossary file '{file_path}'")));
    }
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_GLOSSARY_FILE_BYTES {
        return Err(AppError::InvalidInput(
            "Glossary file exceeds the 10 MB maximum size limit".to_string(),
        ));
    }

    let bytes = std::fs::read(path)?;
    let raw_text = String::from_utf8(bytes)
        .map_err(|_| AppError::InvalidInput("Glossary file must be valid UTF-8 text".to_string()))?;
    let trimmed_bom = raw_text.trim_start_matches('\u{FEFF}').trim_start();

    let is_json = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        || trimmed_bom.starts_with('{')
        || trimmed_bom.starts_with('[');

    let (json_name, rows) = if is_json {
        parse_glossary_json(&raw_text)?
    } else {
        (None, parse_glossary_csv(&raw_text))
    };

    let clean_target_id = target_glossary_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let glossary_id = match clean_target_id {
        Some(existing_id) => {
            let exists = state
                .db
                .list_glossaries()?
                .into_iter()
                .any(|g| g.id == existing_id);
            if !exists {
                return Err(AppError::NotFound(format!("Glossary '{existing_id}'")));
            }
            existing_id
        }
        None => {
            let resolved_name = collection_name
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or(json_name)
                .or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .unwrap_or_else(|| "Imported Glossary".to_string());
            let new_id = uuid::Uuid::new_v4().to_string();
            state.db.create_glossary(&new_id, &resolved_name)?;
            new_id
        }
    };

    let mut imported_count = 0usize;
    let mut skipped_count = 0usize;

    for (raw_src, raw_tgt, case_sensitive) in rows {
        let src = raw_src.trim().to_string();
        let tgt = raw_tgt.trim().to_string();
        if src.is_empty() || tgt.is_empty() {
            skipped_count += 1;
            continue;
        }

        let entry = GlossaryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            glossary_id: glossary_id.clone(),
            source_term: src,
            target_term: tgt,
            case_sensitive,
        };

        match state.db.upsert_glossary_entry(&entry) {
            Ok(()) => imported_count += 1,
            Err(AppError::DbError(msg)) if msg.contains("UNIQUE constraint") => {
                skipped_count += 1;
            }
            Err(other) => return Err(other),
        }
    }

    let glossary = state
        .db
        .list_glossaries()?
        .into_iter()
        .find(|g| g.id == glossary_id)
        .ok_or_else(|| AppError::NotFound(format!("Glossary '{glossary_id}'")))?;

    tracing::info!(imported_count, skipped_count, "Imported glossary file");
    Ok(GlossaryImportResult {
        glossary,
        imported_count,
        skipped_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_glossary_json_and_accepts_camel_or_array() {
        let entries = vec![
            GlossaryEntry {
                id: "1".to_string(),
                glossary_id: "g1".to_string(),
                source_term: "Sandevistan".to_string(),
                target_term: "Sandevistan".to_string(),
                case_sensitive: true,
            },
            GlossaryEntry {
                id: "2".to_string(),
                glossary_id: "g1".to_string(),
                source_term: "Nakama".to_string(),
                target_term: "Rekan".to_string(),
                case_sensitive: false,
            },
        ];
        let json = serialize_glossary_json("Cyberpunk Terms", &entries).expect("serialize json");
        let (name, parsed) = parse_glossary_json(&json).expect("parse json");
        assert_eq!(name.as_deref(), Some("Cyberpunk Terms"));
        assert_eq!(
            parsed,
            vec![
                ("Sandevistan".to_string(), "Sandevistan".to_string(), true),
                ("Nakama".to_string(), "Rekan".to_string(), false),
            ]
        );

        let camel_array = r#"[{"sourceTerm":"Onii-chan","targetTerm":"Kakak","caseSensitive":false}]"#;
        let (arr_name, arr_parsed) = parse_glossary_json(camel_array).expect("parse array");
        assert_eq!(arr_name, None);
        assert_eq!(
            arr_parsed,
            vec![("Onii-chan".to_string(), "Kakak".to_string(), false)]
        );
    }

    #[test]
    fn round_trips_glossary_csv_with_quoted_commas_and_bom() {
        let entries = vec![GlossaryEntry {
            id: "1".to_string(),
            glossary_id: "g1".to_string(),
            source_term: "Hello, \"World\"".to_string(),
            target_term: "Halo, \"Dunia\"".to_string(),
            case_sensitive: true,
        }];
        let csv = serialize_glossary_csv(&entries);
        let with_bom = format!("\u{FEFF}{csv}");
        let parsed = parse_glossary_csv(&with_bom);
        assert_eq!(
            parsed,
            vec![(
                "Hello, \"World\"".to_string(),
                "Halo, \"Dunia\"".to_string(),
                true
            )]
        );
    }
}
