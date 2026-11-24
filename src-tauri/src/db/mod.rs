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
                source_lang: "ja".to_string(),
                target_lang: "id".to_string(),
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
        assert_eq!(settings.auto_idle_unload_minutes, 10);
        assert_eq!(settings.tts_provider, "native");
        settings.display_mode = "compact".to_string();
        settings.backend_override = "cpu".to_string();
        settings.ctx_size_override = "2048".to_string();
        settings.auto_idle_unload_minutes = 15;
        settings.tts_rate = 1.25;
        db.save_settings(&settings).unwrap();

        let reloaded = db.get_settings().unwrap();
        assert_eq!(reloaded, settings);

        drop(db);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_legacy_v1_database_non_destructively_to_v2() {
        let dir = std::env::temp_dir().join(format!("osmium_mig_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("osmium.db");
        let model_dir = dir.join("models");
        let export_dir = dir.join("exports");

        {
            let mut conn = rusqlite::Connection::open(&db_path).unwrap();
            migrations::configure_connection(&conn).unwrap();
            let tx = conn.transaction().unwrap();
            tx.execute_batch(
                "
                CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE installed_models (id TEXT PRIMARY KEY, display_name TEXT NOT NULL, tier TEXT NOT NULL, file_path TEXT NOT NULL, file_size_bytes INTEGER NOT NULL, sha256 TEXT, quantization TEXT, is_byom INTEGER DEFAULT 0, block_count INTEGER, is_active INTEGER DEFAULT 0, created_at INTEGER NOT NULL);
                CREATE TABLE model_downloads (model_id TEXT PRIMARY KEY, download_url TEXT NOT NULL, target_path TEXT NOT NULL, part_path TEXT NOT NULL, expected_sha256 TEXT NOT NULL, etag TEXT, total_bytes INTEGER NOT NULL, bytes_downloaded INTEGER DEFAULT 0, status TEXT NOT NULL, error_msg TEXT, updated_at INTEGER NOT NULL);
                CREATE TABLE text_history (id TEXT PRIMARY KEY, source_text TEXT NOT NULL, translated_text TEXT NOT NULL, tone TEXT NOT NULL, model_id TEXT NOT NULL, model_display_name TEXT NOT NULL, ttft_ms INTEGER, tok_per_sec REAL, created_at INTEGER NOT NULL);
                CREATE TABLE glossaries (id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE glossary_entries (id TEXT PRIMARY KEY, glossary_id TEXT NOT NULL REFERENCES glossaries(id) ON DELETE CASCADE, source_term TEXT NOT NULL, target_term TEXT NOT NULL, case_sensitive INTEGER DEFAULT 0);
                CREATE TABLE subtitle_jobs (id TEXT PRIMARY KEY, file_name TEXT NOT NULL, file_path TEXT NOT NULL, file_size_bytes INTEGER NOT NULL, format TEXT NOT NULL, raw_header TEXT, raw_footer TEXT, ass_format_cols TEXT, genre TEXT NOT NULL, glossary_id TEXT REFERENCES glossaries(id) ON DELETE SET NULL, tag_shield_enabled INTEGER DEFAULT 1, model_id TEXT NOT NULL, model_display_name TEXT NOT NULL, status TEXT NOT NULL, total_cues INTEGER NOT NULL, completed_cues INTEGER DEFAULT 0, self_healing_errors INTEGER DEFAULT 0, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                CREATE TABLE subtitle_cues (job_id TEXT NOT NULL REFERENCES subtitle_jobs(id) ON DELETE CASCADE, cue_index INTEGER NOT NULL, cue_type TEXT NOT NULL DEFAULT 'dialogue', start_time TEXT NOT NULL, end_time TEXT NOT NULL, meta_json TEXT, glossary_matches_json TEXT, original_text TEXT NOT NULL, translated_text TEXT, batch_number INTEGER, status TEXT NOT NULL, warning_msg TEXT, PRIMARY KEY (job_id, cue_index));
                INSERT INTO text_history (id, source_text, translated_text, tone, model_id, model_display_name, ttft_ms, tok_per_sec, created_at)
                VALUES ('h1', 'Hello', 'Halo', 'natural', 'qwen3-8b', 'Qwen 3 8B', 300, 40.0, 123);
                INSERT INTO subtitle_jobs (id, file_name, file_path, file_size_bytes, format, genre, tag_shield_enabled, model_id, model_display_name, status, total_cues, completed_cues, self_healing_errors, created_at, updated_at)
                VALUES ('j1', 'old.srt', 'D:\\old.srt', 100, 'srt', 'film', 1, 'qwen3-8b', 'Qwen 3 8B', 'paused', 10, 5, 0, 100, 200);
                ",
            )
            .unwrap();
            tx.pragma_update(None, "user_version", 1).unwrap();
            tx.commit().unwrap();
        }

        let db = Database::open(&db_path, &model_dir, &export_dir).expect("migrate v1 -> v2");
        let version: i32 = db
            .with_conn(|c| Ok(c.query_row("PRAGMA user_version;", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(version, 2);

        let history = db.list_text_history(None, 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].source_lang, "en");
        assert_eq!(history[0].target_lang, "id");

        let jobs = db.list_subtitle_jobs().unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].source_lang, "en");
        assert_eq!(jobs[0].target_lang, "id");

        let settings = db.get_settings().unwrap();
        assert_eq!(settings.auto_idle_unload_minutes, 10);
        assert_eq!(settings.tts_provider, "native");

        drop(db);
        let _ = fs::remove_dir_all(&dir);
    }
}
