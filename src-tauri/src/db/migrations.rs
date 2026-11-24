use crate::error::AppResult;
use rusqlite::Connection;

pub const CURRENT_SCHEMA_VERSION: i32 = 2;

pub fn configure_connection(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        ",
    )?;
    Ok(())
}

pub fn run_migrations(conn: &mut Connection) -> AppResult<()> {
    configure_connection(conn)?;
    let version: i32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;

    if version < 1 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS installed_models (
                id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                tier TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                sha256 TEXT,
                quantization TEXT,
                is_byom INTEGER DEFAULT 0,
                block_count INTEGER,
                is_active INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS model_downloads (
                model_id TEXT PRIMARY KEY,
                download_url TEXT NOT NULL,
                target_path TEXT NOT NULL,
                part_path TEXT NOT NULL,
                expected_sha256 TEXT NOT NULL,
                etag TEXT,
                total_bytes INTEGER NOT NULL,
                bytes_downloaded INTEGER DEFAULT 0,
                status TEXT NOT NULL,
                error_msg TEXT,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS text_history (
                id TEXT PRIMARY KEY,
                source_text TEXT NOT NULL,
                translated_text TEXT NOT NULL,
                tone TEXT NOT NULL,
                model_id TEXT NOT NULL,
                model_display_name TEXT NOT NULL,
                ttft_ms INTEGER,
                tok_per_sec REAL,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_text_history_created_at
                ON text_history(created_at DESC);

            CREATE TABLE IF NOT EXISTS glossaries (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS glossary_entries (
                id TEXT PRIMARY KEY,
                glossary_id TEXT NOT NULL REFERENCES glossaries(id) ON DELETE CASCADE,
                source_term TEXT NOT NULL,
                target_term TEXT NOT NULL,
                case_sensitive INTEGER DEFAULT 0,
                UNIQUE (glossary_id, source_term, case_sensitive)
            );
            CREATE INDEX IF NOT EXISTS idx_glossary_entries_glossary_id
                ON glossary_entries(glossary_id);

            CREATE TABLE IF NOT EXISTS subtitle_jobs (
                id TEXT PRIMARY KEY,
                file_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                format TEXT NOT NULL,
                raw_header TEXT,
                raw_footer TEXT,
                ass_format_cols TEXT,
                genre TEXT NOT NULL,
                glossary_id TEXT REFERENCES glossaries(id) ON DELETE SET NULL,
                tag_shield_enabled INTEGER DEFAULT 1,
                model_id TEXT NOT NULL,
                model_display_name TEXT NOT NULL,
                status TEXT NOT NULL,
                total_cues INTEGER NOT NULL,
                completed_cues INTEGER DEFAULT 0,
                self_healing_errors INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_subtitle_jobs_updated_at
                ON subtitle_jobs(updated_at DESC);

            CREATE TABLE IF NOT EXISTS subtitle_cues (
                job_id TEXT NOT NULL REFERENCES subtitle_jobs(id) ON DELETE CASCADE,
                cue_index INTEGER NOT NULL,
                cue_type TEXT NOT NULL DEFAULT 'dialogue',
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                meta_json TEXT,
                glossary_matches_json TEXT,
                original_text TEXT NOT NULL,
                translated_text TEXT,
                batch_number INTEGER,
                status TEXT NOT NULL,
                warning_msg TEXT,
                PRIMARY KEY (job_id, cue_index)
            );
            ",
        )?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
    }

    if version < 2 {
        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            ALTER TABLE text_history ADD COLUMN source_lang TEXT NOT NULL DEFAULT 'en';
            ALTER TABLE text_history ADD COLUMN target_lang TEXT NOT NULL DEFAULT 'id';
            ALTER TABLE subtitle_jobs ADD COLUMN source_lang TEXT NOT NULL DEFAULT 'en';
            ALTER TABLE subtitle_jobs ADD COLUMN target_lang TEXT NOT NULL DEFAULT 'id';
            ",
        )?;
        tx.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
        tx.commit()?;
    }

    Ok(())
}
