pub mod migrations;
pub mod models;
pub mod repositories;

use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(
        db_path: &Path,
        default_model_dir: &Path,
        default_export_dir: &Path,
    ) -> AppResult<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(db_path)?;
        migrations::run_migrations(&mut conn)?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.seed_default_settings(
            &default_model_dir.to_string_lossy(),
            &default_export_dir.to_string_lossy(),
        )?;
        Ok(db)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        migrations::run_migrations(&mut conn)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.seed_default_settings("C:\\Users\\Public\\Osmium\\models", "C:\\Users\\Public\\Osmium\\exports")?;
        Ok(db)
    }

    pub fn with_conn<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce(&mut Connection) -> AppResult<T>,
    {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::InternalError("Database mutex poisoned".to_string()))?;
        f(&mut guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::*;
    use crate::db::repositories::now_unix_ms;

    #[test]
    fn db_enforces_foreign_keys_cascade_and_set_null() {
        let db = Database::open_in_memory().expect("open_in_memory");

        let fk_enabled: i32 = db
            .with_conn(|c| Ok(c.query_row("PRAGMA foreign_keys;", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(fk_enabled, 1, "PRAGMA foreign_keys must be ON");

        let glossary = db.create_glossary("g1", "Cyberpunk").unwrap();
        assert_eq!(glossary.name, "Cyberpunk");

        let entry = GlossaryEntry {
            id: "e1".to_string(),
            glossary_id: "g1".to_string(),
            source_term: "Sandevistan".to_string(),
            target_term: "Sandevistan".to_string(),
            case_sensitive: true,
        };
        db.upsert_glossary_entry(&entry).unwrap();

        // Duplicate (glossary_id, source_term, case_sensitive) with a different id must fail UNIQUE.
        let dup_entry = GlossaryEntry {
            id: "e2".to_string(),
            ..entry.clone()
        };
        assert!(
            db.upsert_glossary_entry(&dup_entry).is_err(),
            "UNIQUE constraint must reject duplicate glossary entry"
        );

        let job = SubtitleJobRecord {
            summary: SubtitleJobSummary {
                id: "job1".to_string(),
                file_name: "ep01.ass".to_string(),
                file_path: "D:\\subs\\ep01.ass".to_string(),
                file_size_bytes: 1024,
                format: "ass".to_string(),
                genre: "anime".to_string(),
                glossary_id: Some("g1".to_string()),
                tag_shield_enabled: true,
                model_id: "qwen3-8b-q4km".to_string(),
                model_display_name: "Qwen 3 8B".to_string(),
                status: "paused".to_string(),
                total_cues: 1,
                completed_cues: 0,
                self_healing_errors: 0,
                created_at: now_unix_ms(),
                updated_at: now_unix_ms(),
            },
            raw_header: Some("[Script Info]".to_string()),
            raw_footer: None,
            ass_format_cols: None,
        };
        let cues = vec![NewSubtitleCue {
            cue_index: 0,
            cue_type: "dialogue".to_string(),
            start_time: "0:00:01.00".to_string(),
            end_time: "0:00:03.00".to_string(),
            meta_json: None,
            glossary_matches_json: Some("[\"Sandevistan\"]".to_string()),
            original_text: "Use the Sandevistan!".to_string(),
            translated_text: None,
            status: "queued".to_string(),
        }];
        db.create_subtitle_job(&job, &cues).unwrap();

        // Deleting glossary must CASCADE delete glossary_entries and SET NULL on subtitle_jobs.glossary_id
        db.delete_glossary("g1").unwrap();
        let remaining_entries = db.list_glossary_entries("g1").unwrap();
        assert!(remaining_entries.is_empty(), "glossary_entries must cascade delete");

        let jobs = db.list_subtitle_jobs().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].glossary_id, None, "subtitle_jobs.glossary_id must ON DELETE SET NULL");

        // Deleting subtitle_job must CASCADE delete subtitle_cues
        db.delete_subtitle_job("job1").unwrap();
        let remaining_cues = db.get_subtitle_cues("job1", 0, 50).unwrap();
        assert!(remaining_cues.is_empty(), "subtitle_cues must cascade delete");
    }

    #[test]
    fn db_wal_mode_and_settings_round_trip() {
        let dir = std::env::temp_dir().join(format!("osmium_db_test_{}", uuid::Uuid::new_v4()));
        let db_path = dir.join("osmium.db");
        let model_dir = dir.join("models");
        let export_dir = dir.join("exports");

        let db = Database::open(&db_path, &model_dir, &export_dir).expect("open file db");
        let journal_mode: String = db
            .with_conn(|c| Ok(c.query_row("PRAGMA journal_mode;", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");
        assert!(db.check_wal_synced());

        let mut settings = db.get_settings().unwrap();
        assert_eq!(settings.display_mode, "standard");
        settings.display_mode = "compact".to_string();
        settings.backend_override = "cpu".to_string();
        settings.ctx_size_override = "2048".to_string();
        db.save_settings(&settings).unwrap();

        let reloaded = db.get_settings().unwrap();
        assert_eq!(reloaded, settings);

        drop(db);
        let _ = fs::remove_dir_all(&dir);
    }
}
