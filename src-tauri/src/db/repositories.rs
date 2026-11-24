use crate::db::models::*;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use rusqlite::{params, OptionalExtension, Row};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Database {
    // ── Settings ─────────────────────────────────────────────────────────────

    pub fn seed_default_settings(
        &self,
        default_model_dir: &str,
        default_export_dir: &str,
    ) -> AppResult<()> {
        let now = now_unix_ms();
        let defaults = [
            ("display_mode", "standard"),
            ("default_model_dir", default_model_dir),
            ("default_export_dir", default_export_dir),
            ("backend_override", "auto"),
            ("ctx_size_override", "auto"),
            ("gpu_layers_override", "auto"),
            ("tts_provider", "native"),
            ("tts_rate", "1.0"),
            ("tts_voice_prefs_json", "{}"),
            ("auto_idle_unload_minutes", "10"),
        ];

        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            )?;
            for (k, v) in defaults {
                stmt.execute(params![k, v, now])?;
            }
            Ok(())
        })
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        self.with_conn(|conn| {
            let mut settings = AppSettings {
                display_mode: "standard".to_string(),
                default_model_dir: String::new(),
                default_export_dir: String::new(),
                backend_override: "auto".to_string(),
                ctx_size_override: "auto".to_string(),
                gpu_layers_override: "auto".to_string(),
                tts_provider: "native".to_string(),
                tts_rate: 1.0,
                tts_voice_prefs_json: "{}".to_string(),
                auto_idle_unload_minutes: 10,
            };

            let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;

            for row in rows {
                let (key, value) = row?;
                match key.as_str() {
                    "display_mode" => settings.display_mode = value,
                    "default_model_dir" => settings.default_model_dir = value,
                    "default_export_dir" => settings.default_export_dir = value,
                    "backend_override" => settings.backend_override = value,
                    "ctx_size_override" => settings.ctx_size_override = value,
                    "gpu_layers_override" => settings.gpu_layers_override = value,
                    "tts_provider" => settings.tts_provider = value,
                    "tts_rate" => {
                        if let Ok(parsed) = value.parse::<f64>() {
                            if parsed.is_finite() {
                                settings.tts_rate = parsed;
                            }
                        }
                    }
                    "tts_voice_prefs_json" => settings.tts_voice_prefs_json = value,
                    "auto_idle_unload_minutes" => {
                        if let Ok(parsed) = value.parse::<u32>() {
                            settings.auto_idle_unload_minutes = parsed;
                        }
                    }
                    _ => {}
                }
            }

            Ok(settings)
        })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let now = now_unix_ms();
        let tts_rate_str = format!("{:.2}", settings.tts_rate);
        let auto_idle_str = settings.auto_idle_unload_minutes.to_string();
        let pairs = [
            ("display_mode", settings.display_mode.as_str()),
            ("default_model_dir", settings.default_model_dir.as_str()),
            ("default_export_dir", settings.default_export_dir.as_str()),
            ("backend_override", settings.backend_override.as_str()),
            ("ctx_size_override", settings.ctx_size_override.as_str()),
            ("gpu_layers_override", settings.gpu_layers_override.as_str()),
            ("tts_provider", settings.tts_provider.as_str()),
            ("tts_rate", tts_rate_str.as_str()),
            ("tts_voice_prefs_json", settings.tts_voice_prefs_json.as_str()),
            ("auto_idle_unload_minutes", auto_idle_str.as_str()),
        ];

        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO app_settings (key, value, updated_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                )?;
                for (k, v) in pairs {
                    stmt.execute(params![k, v, now])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    // ── Installed Models ─────────────────────────────────────────────────────

    pub fn list_installed_models(&self) -> AppResult<Vec<InstalledModel>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, display_name, tier, file_path, file_size_bytes, sha256,
                        quantization, is_byom, block_count, is_active, created_at
                 FROM installed_models
                 ORDER BY is_active DESC, created_at DESC",
            )?;
            let rows = stmt.query_map([], map_installed_model)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn get_active_model(&self) -> AppResult<Option<InstalledModel>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, display_name, tier, file_path, file_size_bytes, sha256,
                        quantization, is_byom, block_count, is_active, created_at
                 FROM installed_models
                 WHERE is_active = 1
                 LIMIT 1",
                [],
                map_installed_model,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    pub fn upsert_installed_model(&self, model: &InstalledModel) -> AppResult<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            if model.is_active {
                tx.execute("UPDATE installed_models SET is_active = 0", [])?;
            }
            tx.execute(
                "INSERT INTO installed_models (
                    id, display_name, tier, file_path, file_size_bytes, sha256,
                    quantization, is_byom, block_count, is_active, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(id) DO UPDATE SET
                    display_name = excluded.display_name,
                    tier = excluded.tier,
                    file_path = excluded.file_path,
                    file_size_bytes = excluded.file_size_bytes,
                    sha256 = excluded.sha256,
                    quantization = excluded.quantization,
                    is_byom = excluded.is_byom,
                    block_count = excluded.block_count,
                    is_active = excluded.is_active",
                params![
                    model.id,
                    model.display_name,
                    model.tier,
                    model.file_path,
                    model.file_size_bytes,
                    model.sha256,
                    model.quantization,
                    model.is_byom as i32,
                    model.block_count,
                    model.is_active as i32,
                    model.created_at,
                ],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn activate_installed_model(&self, model_id: &str) -> AppResult<InstalledModel> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let exists: Option<String> = tx
                .query_row(
                    "SELECT id FROM installed_models WHERE id = ?1",
                    params![model_id],
                    |row| row.get(0),
                )
                .optional()?;

            if exists.is_none() {
                return Err(AppError::NotFound(format!("Model '{model_id}'")));
            }

            tx.execute("UPDATE installed_models SET is_active = 0", [])?;
            tx.execute(
                "UPDATE installed_models SET is_active = 1 WHERE id = ?1",
                params![model_id],
            )?;
            let updated = tx.query_row(
                "SELECT id, display_name, tier, file_path, file_size_bytes, sha256,
                        quantization, is_byom, block_count, is_active, created_at
                 FROM installed_models WHERE id = ?1",
                params![model_id],
                map_installed_model,
            )?;
            tx.commit()?;
            Ok(updated)
        })
    }

    pub fn get_installed_model(&self, model_id: &str) -> AppResult<Option<InstalledModel>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, display_name, tier, file_path, file_size_bytes, sha256,
                        quantization, is_byom, block_count, is_active, created_at
                 FROM installed_models
                 WHERE id = ?1",
                params![model_id],
                map_installed_model,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    pub fn update_model_file_path(
        &self,
        model_id: &str,
        file_path: &str,
        file_size_bytes: i64,
        block_count: Option<i64>,
    ) -> AppResult<InstalledModel> {
        self.with_conn(|conn| {
            let rows_affected = conn.execute(
                "UPDATE installed_models
                 SET file_path = ?1, file_size_bytes = ?2, block_count = ?3
                 WHERE id = ?4",
                params![file_path, file_size_bytes, block_count, model_id],
            )?;
            if rows_affected == 0 {
                return Err(AppError::NotFound(format!("Model '{model_id}'")));
            }
            conn.query_row(
                "SELECT id, display_name, tier, file_path, file_size_bytes, sha256,
                        quantization, is_byom, block_count, is_active, created_at
                 FROM installed_models WHERE id = ?1",
                params![model_id],
                map_installed_model,
            )
            .map_err(AppError::from)
        })
    }

    pub fn delete_installed_model(&self, model_id: &str) -> AppResult<Option<InstalledModel>> {
        self.with_conn(|conn| {
            let model = conn
                .query_row(
                    "SELECT id, display_name, tier, file_path, file_size_bytes, sha256,
                            quantization, is_byom, block_count, is_active, created_at
                     FROM installed_models WHERE id = ?1",
                    params![model_id],
                    map_installed_model,
                )
                .optional()?;

            if model.is_some() {
                conn.execute("DELETE FROM installed_models WHERE id = ?1", params![model_id])?;
            }
            Ok(model)
        })
    }

    // ── Model Downloads ──────────────────────────────────────────────────────

    pub fn list_download_tasks(&self) -> AppResult<Vec<DownloadTask>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT model_id, download_url, target_path, part_path, expected_sha256,
                        etag, total_bytes, bytes_downloaded, status, error_msg, updated_at
                 FROM model_downloads
                 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], map_download_task)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn get_download_task(&self, model_id: &str) -> AppResult<Option<DownloadTask>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT model_id, download_url, target_path, part_path, expected_sha256,
                        etag, total_bytes, bytes_downloaded, status, error_msg, updated_at
                 FROM model_downloads WHERE model_id = ?1",
                params![model_id],
                map_download_task,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    pub fn upsert_download_task(&self, task: &DownloadTask) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO model_downloads (
                    model_id, download_url, target_path, part_path, expected_sha256,
                    etag, total_bytes, bytes_downloaded, status, error_msg, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(model_id) DO UPDATE SET
                    download_url = excluded.download_url,
                    target_path = excluded.target_path,
                    part_path = excluded.part_path,
                    expected_sha256 = excluded.expected_sha256,
                    etag = excluded.etag,
                    total_bytes = excluded.total_bytes,
                    bytes_downloaded = excluded.bytes_downloaded,
                    status = excluded.status,
                    error_msg = excluded.error_msg,
                    updated_at = excluded.updated_at",
                params![
                    task.model_id,
                    task.download_url,
                    task.target_path,
                    task.part_path,
                    task.expected_sha256,
                    task.etag,
                    task.total_bytes,
                    task.bytes_downloaded,
                    task.status,
                    task.error_msg,
                    task.updated_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn delete_download_task(&self, model_id: &str) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM model_downloads WHERE model_id = ?1",
                params![model_id],
            )?;
            Ok(())
        })
    }

    // ── Text History ─────────────────────────────────────────────────────────

    pub fn insert_text_history(&self, item: &TextHistoryItem) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO text_history (
                    id, source_text, translated_text, source_lang, target_lang, tone,
                    model_id, model_display_name, ttft_ms, tok_per_sec, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    item.id,
                    item.source_text,
                    item.translated_text,
                    item.source_lang,
                    item.target_lang,
                    item.tone,
                    item.model_id,
                    item.model_display_name,
                    item.ttft_ms,
                    item.tok_per_sec,
                    item.created_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_text_history(
        &self,
        query: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<TextHistoryItem>> {
        self.with_conn(|conn| {
            let limit_i64 = i64::from(limit.clamp(1, 500));
            if let Some(q) = query.map(str::trim).filter(|s| !s.is_empty()) {
                let pattern = format!("%{}%", escape_like(q));
                let mut stmt = conn.prepare(
                    "SELECT id, source_text, translated_text, source_lang, target_lang, tone,
                            model_id, model_display_name, ttft_ms, tok_per_sec, created_at
                     FROM text_history
                     WHERE source_text LIKE ?1 ESCAPE '\\'
                        OR translated_text LIKE ?1 ESCAPE '\\'
                     ORDER BY created_at DESC
                     LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![pattern, limit_i64], map_text_history)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, source_text, translated_text, source_lang, target_lang, tone,
                            model_id, model_display_name, ttft_ms, tok_per_sec, created_at
                     FROM text_history
                     ORDER BY created_at DESC
                     LIMIT ?1",
                )?;
                let rows = stmt.query_map(params![limit_i64], map_text_history)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
            }
        })
    }

    pub fn delete_text_history(&self, id: &str) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM text_history WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn get_latest_text_history(&self) -> AppResult<Option<TextHistoryItem>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, source_text, translated_text, source_lang, target_lang, tone,
                        model_id, model_display_name, ttft_ms, tok_per_sec, created_at
                 FROM text_history
                 ORDER BY created_at DESC
                 LIMIT 1",
                [],
                map_text_history,
            )
            .optional()
            .map_err(AppError::from)
        })
    }

    // ── Glossaries ───────────────────────────────────────────────────────────

    pub fn create_glossary(&self, id: &str, name: &str) -> AppResult<Glossary> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO glossaries (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, now, now],
            )?;
            Ok(Glossary {
                id: id.to_string(),
                name: name.to_string(),
                entry_count: 0,
                created_at: now,
                updated_at: now,
            })
        })
    }

    pub fn list_glossaries(&self) -> AppResult<Vec<Glossary>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT g.id, g.name, COUNT(e.id) AS entry_count, g.created_at, g.updated_at
                 FROM glossaries g
                 LEFT JOIN glossary_entries e ON e.glossary_id = g.id
                 GROUP BY g.id
                 ORDER BY g.updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(Glossary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entry_count: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn update_glossary_name(&self, id: &str, name: &str) -> AppResult<Glossary> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let affected = conn.execute(
                "UPDATE glossaries SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now, id],
            )?;
            if affected == 0 {
                return Err(AppError::NotFound(format!("Glossary '{id}'")));
            }
            conn.query_row(
                "SELECT g.id, g.name, COUNT(e.id) AS entry_count, g.created_at, g.updated_at
                 FROM glossaries g
                 LEFT JOIN glossary_entries e ON e.glossary_id = g.id
                 WHERE g.id = ?1
                 GROUP BY g.id",
                params![id],
                |row| {
                    Ok(Glossary {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        entry_count: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .map_err(AppError::from)
        })
    }

    pub fn delete_glossary(&self, id: &str) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM glossaries WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn delete_glossary_entry(&self, entry_id: &str, glossary_id: &str) -> AppResult<()> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "DELETE FROM glossary_entries WHERE id = ?1 AND glossary_id = ?2",
                params![entry_id, glossary_id],
            )?;
            tx.execute(
                "UPDATE glossaries SET updated_at = ?1 WHERE id = ?2",
                params![now, glossary_id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn upsert_glossary_entry(&self, entry: &GlossaryEntry) -> AppResult<()> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO glossary_entries (id, glossary_id, source_term, target_term, case_sensitive)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    source_term = excluded.source_term,
                    target_term = excluded.target_term,
                    case_sensitive = excluded.case_sensitive",
                params![
                    entry.id,
                    entry.glossary_id,
                    entry.source_term,
                    entry.target_term,
                    entry.case_sensitive as i32,
                ],
            )?;
            tx.execute(
                "UPDATE glossaries SET updated_at = ?1 WHERE id = ?2",
                params![now, entry.glossary_id],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn list_glossary_entries(&self, glossary_id: &str) -> AppResult<Vec<GlossaryEntry>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, glossary_id, source_term, target_term, case_sensitive
                 FROM glossary_entries
                 WHERE glossary_id = ?1
                 ORDER BY source_term COLLATE NOCASE ASC",
            )?;
            let rows = stmt.query_map(params![glossary_id], |row| {
                Ok(GlossaryEntry {
                    id: row.get(0)?,
                    glossary_id: row.get(1)?,
                    source_term: row.get(2)?,
                    target_term: row.get(3)?,
                    case_sensitive: row.get::<_, i32>(4)? != 0,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    // ── Subtitle Jobs & Cues ─────────────────────────────────────────────────

    pub fn create_subtitle_job(
        &self,
        job: &SubtitleJobRecord,
        cues: &[NewSubtitleCue],
    ) -> AppResult<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "INSERT INTO subtitle_jobs (
                    id, file_name, file_path, file_size_bytes, format,
                    source_lang, target_lang, raw_header, raw_footer, ass_format_cols,
                    genre, glossary_id, tag_shield_enabled, model_id, model_display_name,
                    status, total_cues, completed_cues, self_healing_errors, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                params![
                    job.summary.id,
                    job.summary.file_name,
                    job.summary.file_path,
                    job.summary.file_size_bytes,
                    job.summary.format,
                    job.summary.source_lang,
                    job.summary.target_lang,
                    job.raw_header,
                    job.raw_footer,
                    job.ass_format_cols,
                    job.summary.genre,
                    job.summary.glossary_id,
                    job.summary.tag_shield_enabled as i32,
                    job.summary.model_id,
                    job.summary.model_display_name,
                    job.summary.status,
                    job.summary.total_cues,
                    job.summary.completed_cues,
                    job.summary.self_healing_errors,
                    job.summary.created_at,
                    job.summary.updated_at,
                ],
            )?;

            {
                let mut stmt = tx.prepare(
                    "INSERT INTO subtitle_cues (
                        job_id, cue_index, cue_type, start_time, end_time,
                        meta_json, glossary_matches_json, original_text, translated_text, status
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )?;
                for cue in cues {
                    stmt.execute(params![
                        job.summary.id,
                        cue.cue_index,
                        cue.cue_type,
                        cue.start_time,
                        cue.end_time,
                        cue.meta_json,
                        cue.glossary_matches_json,
                        cue.original_text,
                        cue.translated_text,
                        cue.status,
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn list_subtitle_jobs(&self) -> AppResult<Vec<SubtitleJobSummary>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, file_name, file_path, file_size_bytes, format,
                        source_lang, target_lang, genre,
                        glossary_id, tag_shield_enabled, model_id, model_display_name,
                        status, total_cues, completed_cues, self_healing_errors,
                        created_at, updated_at
                 FROM subtitle_jobs
                 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], map_subtitle_job_summary)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn delete_subtitle_job(&self, job_id: &str) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM subtitle_jobs WHERE id = ?1", params![job_id])?;
            Ok(())
        })
    }

    pub fn get_subtitle_cues(
        &self,
        job_id: &str,
        offset: u32,
        limit: u32,
    ) -> AppResult<Vec<SubtitleCueDto>> {
        self.get_subtitle_cues_filtered(job_id, offset, limit, None, None)
    }

    pub fn get_subtitle_cues_filtered(
        &self,
        job_id: &str,
        offset: u32,
        limit: u32,
        search_query: Option<&str>,
        status_filter: Option<&str>,
    ) -> AppResult<Vec<SubtitleCueDto>> {
        self.with_conn(|conn| {
            let trimmed_search = search_query.map(str::trim).filter(|s| !s.is_empty());
            let trimmed_status = status_filter
                .map(str::trim)
                .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"));

            let mut sql = String::from(
                "SELECT job_id, cue_index, cue_type, start_time, end_time,
                        meta_json, glossary_matches_json, original_text, translated_text,
                        batch_number, status, warning_msg
                 FROM subtitle_cues
                 WHERE job_id = ?1",
            );
            let mut bind_values: Vec<rusqlite::types::Value> =
                vec![rusqlite::types::Value::Text(job_id.to_string())];

            if let Some(status_val) = trimmed_status {
                if status_val.eq_ignore_ascii_case("warning") {
                    sql.push_str(" AND status = 'warning'");
                } else {
                    sql.push_str(&format!(" AND status = ?{}", bind_values.len() + 1));
                    bind_values.push(rusqlite::types::Value::Text(status_val.to_string()));
                }
            }

            if let Some(q) = trimmed_search {
                let pattern = format!("%{}%", escape_like(q));
                let idx = bind_values.len() + 1;
                sql.push_str(&format!(
                    " AND (original_text LIKE ?{idx} ESCAPE '\\'
                           OR COALESCE(translated_text, '') LIKE ?{idx} ESCAPE '\\'
                           OR start_time LIKE ?{idx} ESCAPE '\\'
                           OR end_time LIKE ?{idx} ESCAPE '\\'
                           OR CAST(cue_index AS TEXT) LIKE ?{idx} ESCAPE '\\')"
                ));
                bind_values.push(rusqlite::types::Value::Text(pattern));
            }

            let limit_idx = bind_values.len() + 1;
            let offset_idx = bind_values.len() + 2;
            sql.push_str(&format!(
                " ORDER BY cue_index ASC LIMIT ?{limit_idx} OFFSET ?{offset_idx}"
            ));
            bind_values.push(rusqlite::types::Value::Integer(i64::from(
                limit.clamp(1, 5000),
            )));
            bind_values.push(rusqlite::types::Value::Integer(i64::from(offset)));

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(
                rusqlite::params_from_iter(bind_values.iter()),
                map_subtitle_cue,
            )?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn get_all_subtitle_cues(&self, job_id: &str) -> AppResult<Vec<SubtitleCueDto>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT job_id, cue_index, cue_type, start_time, end_time,
                        meta_json, glossary_matches_json, original_text, translated_text,
                        batch_number, status, warning_msg
                 FROM subtitle_cues
                 WHERE job_id = ?1
                 ORDER BY cue_index ASC",
            )?;
            let rows = stmt.query_map(params![job_id], map_subtitle_cue)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn get_single_subtitle_cue(
        &self,
        job_id: &str,
        cue_index: i64,
    ) -> AppResult<SubtitleCueDto> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT job_id, cue_index, cue_type, start_time, end_time,
                        meta_json, glossary_matches_json, original_text, translated_text,
                        batch_number, status, warning_msg
                 FROM subtitle_cues
                 WHERE job_id = ?1 AND cue_index = ?2",
                params![job_id, cue_index],
                map_subtitle_cue,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!(
                    "Subtitle cue #{cue_index} not found for job {job_id}"
                )),
                other => AppError::from(other),
            })
        })
    }

    pub fn get_subtitle_job_record(&self, job_id: &str) -> AppResult<SubtitleJobRecord> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, file_name, file_path, file_size_bytes, format,
                        source_lang, target_lang, genre,
                        glossary_id, tag_shield_enabled, model_id, model_display_name,
                        status, total_cues, completed_cues, self_healing_errors,
                        created_at, updated_at, raw_header, raw_footer, ass_format_cols
                 FROM subtitle_jobs
                 WHERE id = ?1",
                params![job_id],
                |row| {
                    let summary = map_subtitle_job_summary(row)?;
                    Ok(SubtitleJobRecord {
                        summary,
                        raw_header: row.get(18)?,
                        raw_footer: row.get(19)?,
                        ass_format_cols: row.get(20)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("Subtitle job {job_id} not found"))
                }
                other => AppError::from(other),
            })
        })
    }

    pub fn get_subtitle_job_summary(&self, job_id: &str) -> AppResult<SubtitleJobSummary> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, file_name, file_path, file_size_bytes, format,
                        source_lang, target_lang, genre,
                        glossary_id, tag_shield_enabled, model_id, model_display_name,
                        status, total_cues, completed_cues, self_healing_errors,
                        created_at, updated_at
                 FROM subtitle_jobs
                 WHERE id = ?1",
                params![job_id],
                map_subtitle_job_summary,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::NotFound(format!("Subtitle job {job_id} not found"))
                }
                other => AppError::from(other),
            })
        })
    }

    pub fn update_subtitle_job_options(
        &self,
        job_id: &str,
        source_lang: &str,
        target_lang: &str,
        genre: &str,
        glossary_id: Option<&str>,
        tag_shield_enabled: bool,
    ) -> AppResult<SubtitleJobSummary> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE subtitle_jobs
                 SET source_lang = ?1, target_lang = ?2, genre = ?3, glossary_id = ?4, tag_shield_enabled = ?5, updated_at = ?6
                 WHERE id = ?7",
                params![
                    source_lang,
                    target_lang,
                    genre,
                    glossary_id,
                    tag_shield_enabled as i32,
                    now,
                    job_id
                ],
            )?;
            if changed == 0 {
                return Err(AppError::NotFound(format!("Subtitle job {job_id} not found")));
            }
            Ok(())
        })?;
        self.get_subtitle_job_summary(job_id)
    }

    pub fn update_subtitle_job_model_and_status(
        &self,
        job_id: &str,
        model_id: &str,
        model_display_name: &str,
        status: &str,
    ) -> AppResult<()> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE subtitle_jobs
                 SET model_id = ?1, model_display_name = ?2, status = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![model_id, model_display_name, status, now, job_id],
            )?;
            if changed == 0 {
                return Err(AppError::NotFound(format!("Subtitle job {job_id} not found")));
            }
            Ok(())
        })
    }

    pub fn update_subtitle_job_status(&self, job_id: &str, status: &str) -> AppResult<()> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE subtitle_jobs SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status, now, job_id],
            )?;
            Ok(())
        })
    }

    pub fn recover_stale_running_jobs(&self) -> AppResult<()> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "UPDATE subtitle_cues SET status = 'queued' WHERE status = 'processing'",
                [],
            )?;
            tx.execute(
                "UPDATE subtitle_jobs SET status = 'paused', updated_at = ?1 WHERE status = 'running'",
                params![now],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    pub fn mark_cues_processing(
        &self,
        job_id: &str,
        cue_indices: &[i64],
        batch_number: i64,
    ) -> AppResult<()> {
        if cue_indices.is_empty() {
            return Ok(());
        }
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "UPDATE subtitle_cues
                     SET status = 'processing', batch_number = ?1
                     WHERE job_id = ?2 AND cue_index = ?3",
                )?;
                for cue_idx in cue_indices {
                    stmt.execute(params![batch_number, job_id, cue_idx])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn reset_processing_cues_to_queued(&self, job_id: &str) -> AppResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE subtitle_cues SET status = 'queued'
                 WHERE job_id = ?1 AND status = 'processing'",
                params![job_id],
            )?;
            Ok(())
        })
    }

    pub fn commit_batch_cues(
        &self,
        job_id: &str,
        batch_number: i64,
        updates: &[(i64, String, String, Option<String>, Option<String>)],
        next_job_status: Option<&str>,
    ) -> AppResult<(SubtitleJobSummary, Vec<SubtitleCueDto>)> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "UPDATE subtitle_cues
                     SET translated_text = ?1,
                         status = ?2,
                         warning_msg = ?3,
                         glossary_matches_json = ?4,
                         batch_number = ?5
                     WHERE job_id = ?6 AND cue_index = ?7",
                )?;
                for (cue_index, translated_text, status, warning_msg, glossary_json) in updates {
                    stmt.execute(params![
                        translated_text,
                        status,
                        warning_msg,
                        glossary_json,
                        batch_number,
                        job_id,
                        cue_index
                    ])?;
                }
            }

            let (completed_cues, self_healing_errors): (i64, i64) = tx.query_row(
                "SELECT
                    COALESCE(SUM(CASE WHEN cue_type = 'dialogue' AND status IN ('done', 'warning') THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'warning' THEN 1 ELSE 0 END), 0)
                 FROM subtitle_cues
                 WHERE job_id = ?1",
                params![job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

            if let Some(status_val) = next_job_status {
                tx.execute(
                    "UPDATE subtitle_jobs
                     SET completed_cues = ?1, self_healing_errors = ?2, status = ?3, updated_at = ?4
                     WHERE id = ?5",
                    params![completed_cues, self_healing_errors, status_val, now, job_id],
                )?;
            } else {
                tx.execute(
                    "UPDATE subtitle_jobs
                     SET completed_cues = ?1, self_healing_errors = ?2, updated_at = ?3
                     WHERE id = ?4",
                    params![completed_cues, self_healing_errors, now, job_id],
                )?;
            }

            let mut updated_cues = Vec::with_capacity(updates.len());
            {
                let mut cue_stmt = tx.prepare(
                    "SELECT job_id, cue_index, cue_type, start_time, end_time,
                            meta_json, glossary_matches_json, original_text, translated_text,
                            batch_number, status, warning_msg
                     FROM subtitle_cues
                     WHERE job_id = ?1 AND cue_index = ?2",
                )?;
                for (cue_index, _, _, _, _) in updates {
                    if let Ok(dto) = cue_stmt.query_row(params![job_id, cue_index], map_subtitle_cue) {
                        updated_cues.push(dto);
                    }
                }
            }

            let summary = tx.query_row(
                "SELECT id, file_name, file_path, file_size_bytes, format,
                        source_lang, target_lang, genre,
                        glossary_id, tag_shield_enabled, model_id, model_display_name,
                        status, total_cues, completed_cues, self_healing_errors,
                        created_at, updated_at
                 FROM subtitle_jobs
                 WHERE id = ?1",
                params![job_id],
                map_subtitle_job_summary,
            )?;

            tx.commit()?;
            Ok((summary, updated_cues))
        })
    }

    pub fn update_single_cue_and_recompute_job(
        &self,
        job_id: &str,
        cue_index: i64,
        translated_text: &str,
        status: &str,
        warning_msg: Option<&str>,
        glossary_matches_json: Option<&str>,
    ) -> AppResult<SubtitleCueDto> {
        let now = now_unix_ms();
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let changed = tx.execute(
                "UPDATE subtitle_cues
                 SET translated_text = ?1,
                     status = ?2,
                     warning_msg = ?3,
                     glossary_matches_json = COALESCE(?4, glossary_matches_json)
                 WHERE job_id = ?5 AND cue_index = ?6",
                params![
                    translated_text,
                    status,
                    warning_msg,
                    glossary_matches_json,
                    job_id,
                    cue_index
                ],
            )?;
            if changed == 0 {
                return Err(AppError::NotFound(format!(
                    "Subtitle cue #{cue_index} not found in job {job_id}"
                )));
            }

            let (completed_cues, self_healing_errors): (i64, i64) = tx.query_row(
                "SELECT
                    COALESCE(SUM(CASE WHEN cue_type = 'dialogue' AND status IN ('done', 'warning') THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN status = 'warning' THEN 1 ELSE 0 END), 0)
                 FROM subtitle_cues
                 WHERE job_id = ?1",
                params![job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            tx.execute(
                "UPDATE subtitle_jobs
                 SET completed_cues = ?1, self_healing_errors = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![completed_cues, self_healing_errors, now, job_id],
            )?;

            let dto = tx.query_row(
                "SELECT job_id, cue_index, cue_type, start_time, end_time,
                        meta_json, glossary_matches_json, original_text, translated_text,
                        batch_number, status, warning_msg
                 FROM subtitle_cues
                 WHERE job_id = ?1 AND cue_index = ?2",
                params![job_id, cue_index],
                map_subtitle_cue,
            )?;

            tx.commit()?;
            Ok(dto)
        })
    }

    pub fn get_previous_dialogue_context(
        &self,
        job_id: &str,
        before_cue_index: i64,
        limit: usize,
    ) -> AppResult<Vec<(String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT original_text, translated_text
                 FROM subtitle_cues
                 WHERE job_id = ?1
                   AND cue_type = 'dialogue'
                   AND cue_index < ?2
                   AND status = 'done'
                   AND translated_text IS NOT NULL
                 ORDER BY cue_index DESC
                 LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![job_id, before_cue_index, limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut pairs = rows.collect::<Result<Vec<_>, _>>()?;
            pairs.reverse();
            Ok(pairs)
        })
    }

    pub fn get_following_dialogue_context(
        &self,
        job_id: &str,
        after_cue_index: i64,
        limit: usize,
    ) -> AppResult<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT original_text
                 FROM subtitle_cues
                 WHERE job_id = ?1
                   AND cue_type = 'dialogue'
                   AND cue_index > ?2
                 ORDER BY cue_index ASC
                 LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![job_id, after_cue_index, limit as i64], |row| {
                row.get::<_, String>(0)
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn check_wal_synced(&self) -> bool {
        self.with_conn(|conn| {
            let res: rusqlite::Result<(i32, i32, i32)> = conn.query_row(
                "PRAGMA wal_checkpoint(PASSIVE)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            );
            match res {
                Ok((busy, _, _)) => Ok(busy == 0),
                Err(_) => Ok(false),
            }
        })
        .unwrap_or(false)
    }
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn map_installed_model(row: &Row<'_>) -> rusqlite::Result<InstalledModel> {
    Ok(InstalledModel {
        id: row.get(0)?,
        display_name: row.get(1)?,
        tier: row.get(2)?,
        file_path: row.get(3)?,
        file_size_bytes: row.get(4)?,
        sha256: row.get(5)?,
        quantization: row.get(6)?,
        is_byom: row.get::<_, i32>(7)? != 0,
        block_count: row.get(8)?,
        is_active: row.get::<_, i32>(9)? != 0,
        created_at: row.get(10)?,
    })
}

fn map_download_task(row: &Row<'_>) -> rusqlite::Result<DownloadTask> {
    Ok(DownloadTask {
        model_id: row.get(0)?,
        download_url: row.get(1)?,
        target_path: row.get(2)?,
        part_path: row.get(3)?,
        expected_sha256: row.get(4)?,
        etag: row.get(5)?,
        total_bytes: row.get(6)?,
        bytes_downloaded: row.get(7)?,
        status: row.get(8)?,
        error_msg: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_text_history(row: &Row<'_>) -> rusqlite::Result<TextHistoryItem> {
    Ok(TextHistoryItem {
        id: row.get(0)?,
        source_text: row.get(1)?,
        translated_text: row.get(2)?,
        source_lang: row.get(3)?,
        target_lang: row.get(4)?,
        tone: row.get(5)?,
        model_id: row.get(6)?,
        model_display_name: row.get(7)?,
        ttft_ms: row.get(8)?,
        tok_per_sec: row.get(9)?,
        created_at: row.get(10)?,
    })
}

fn map_subtitle_job_summary(row: &Row<'_>) -> rusqlite::Result<SubtitleJobSummary> {
    Ok(SubtitleJobSummary {
        id: row.get(0)?,
        file_name: row.get(1)?,
        file_path: row.get(2)?,
        file_size_bytes: row.get(3)?,
        format: row.get(4)?,
        source_lang: row.get(5)?,
        target_lang: row.get(6)?,
        genre: row.get(7)?,
        glossary_id: row.get(8)?,
        tag_shield_enabled: row.get::<_, i32>(9)? != 0,
        model_id: row.get(10)?,
        model_display_name: row.get(11)?,
        status: row.get(12)?,
        total_cues: row.get(13)?,
        completed_cues: row.get(14)?,
        self_healing_errors: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
    })
}

fn map_subtitle_cue(row: &Row<'_>) -> rusqlite::Result<SubtitleCueDto> {
    Ok(SubtitleCueDto {
        job_id: row.get(0)?,
        cue_index: row.get(1)?,
        cue_type: row.get(2)?,
        start_time: row.get(3)?,
        end_time: row.get(4)?,
        meta_json: row.get(5)?,
        glossary_matches_json: row.get(6)?,
        original_text: row.get(7)?,
        translated_text: row.get(8)?,
        batch_number: row.get(9)?,
        status: row.get(10)?,
        warning_msg: row.get(11)?,
    })
}
