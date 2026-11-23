use crate::db::models::{Glossary, GlossaryEntry};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use tauri::State;

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
