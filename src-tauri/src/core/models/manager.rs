use crate::core::hardware::disk;
use crate::core::models::catalog::{official_catalog, CatalogModel};
use crate::core::models::gguf_validator::validate_gguf_file;
use crate::db::models::{DownloadTask, InstalledModel};
use crate::db::repositories::now_unix_ms;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledModelWithHealth {
    #[serde(flatten)]
    pub model: InstalledModel,
    pub file_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsOverview {
    pub catalog: Vec<CatalogModel>,
    pub installed: Vec<InstalledModelWithHealth>,
    pub downloads: Vec<DownloadTask>,
}

pub fn get_models_overview(db: &Database) -> AppResult<ModelsOverview> {
    let installed_raw = db.list_installed_models()?;
    let installed = installed_raw
        .into_iter()
        .map(|model| {
            let file_exists = Path::new(&model.file_path).is_file();
            InstalledModelWithHealth { model, file_exists }
        })
        .collect();

    let downloads = db.list_download_tasks()?;
    Ok(ModelsOverview {
        catalog: official_catalog(),
        installed,
        downloads,
    })
}

pub async fn import_local_gguf(
    state: &AppState,
    file_path: &str,
) -> AppResult<InstalledModelWithHealth> {
    let canonical = disk::validate_readable_file(&PathBuf::from(file_path))?;
    let is_gguf_ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gguf"))
        .unwrap_or(false);
    if !is_gguf_ext {
        return Err(AppError::InvalidInput(
            "Selected file must have a .gguf extension".to_string(),
        ));
    }

    let path_clone = canonical.clone();
    let meta = tokio::task::spawn_blocking(move || validate_gguf_file(&path_clone))
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))??;

    let canonical_str = canonical.to_string_lossy().to_string();
    let existing = state
        .db
        .list_installed_models()?
        .into_iter()
        .find(|m| m.file_path.eq_ignore_ascii_case(&canonical_str));

    if let Some(found) = existing {
        return Ok(InstalledModelWithHealth {
            model: found,
            file_exists: true,
        });
    }

    let should_activate = state.db.get_active_model()?.is_none();
    let model = InstalledModel {
        id: format!("byom-{}", uuid::Uuid::new_v4()),
        display_name: meta.display_name,
        tier: "custom_byom".to_string(),
        file_path: canonical_str,
        file_size_bytes: meta.file_size_bytes as i64,
        sha256: None,
        quantization: meta.quantization,
        is_byom: true,
        block_count: meta.block_count.map(i64::from),
        is_active: should_activate,
        created_at: now_unix_ms(),
    };

    state.db.upsert_installed_model(&model)?;
    if model.is_active {
        sync_standby_for_active(state, &model).await;
    }

    tracing::info!(model_id = %model.id, is_byom = true, "Imported local BYOM GGUF reference");
    Ok(InstalledModelWithHealth {
        model,
        file_exists: true,
    })
}

pub async fn relink_byom_model(
    state: &AppState,
    model_id: &str,
    new_file_path: &str,
) -> AppResult<InstalledModelWithHealth> {
    state.coordinator.ensure_idle_for_admin_action().await?;
    let existing = state
        .db
        .get_installed_model(model_id)?
        .ok_or_else(|| AppError::NotFound(format!("Model '{model_id}'")))?;
    if !existing.is_byom {
        return Err(AppError::InvalidInput(
            "Only local BYOM models can be relinked".to_string(),
        ));
    }

    let canonical = disk::validate_readable_file(&PathBuf::from(new_file_path))?;
    let path_clone = canonical.clone();
    let meta = tokio::task::spawn_blocking(move || validate_gguf_file(&path_clone))
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))??;

    let updated = state.db.update_model_file_path(
        model_id,
        &canonical.to_string_lossy(),
        meta.file_size_bytes as i64,
        meta.block_count.map(i64::from),
    )?;

    if updated.is_active {
        state.sidecar.stop(true).await;
        sync_standby_for_active(state, &updated).await;
    }

    Ok(InstalledModelWithHealth {
        model: updated,
        file_exists: true,
    })
}

pub async fn activate_model(state: &AppState, model_id: &str) -> AppResult<InstalledModelWithHealth> {
    state.coordinator.ensure_idle_for_admin_action().await?;
    let target = state
        .db
        .get_installed_model(model_id)?
        .ok_or_else(|| AppError::NotFound(format!("Model '{model_id}'")))?;

    if !Path::new(&target.file_path).is_file() {
        return Err(AppError::ByomFileMissing(target.file_path));
    }

    state.sidecar.stop(true).await;
    let activated = state.db.activate_installed_model(model_id)?;
    sync_standby_for_active(state, &activated).await;

    tracing::info!(model_id = %activated.id, "Activated local model (standby lazy load)");
    Ok(InstalledModelWithHealth {
        model: activated,
        file_exists: true,
    })
}

pub async fn delete_model(state: &AppState, model_id: &str) -> AppResult<()> {
    state.coordinator.ensure_idle_for_admin_action().await?;
    let Some(existing) = state.db.get_installed_model(model_id)? else {
        return Err(AppError::NotFound(format!("Model '{model_id}'")));
    };

    if existing.is_active {
        state.sidecar.stop(false).await;
    }

    state.db.delete_installed_model(model_id)?;

    // Official models are removed from disk; BYOM models only remove the path reference.
    if !existing.is_byom {
        let path = Path::new(&existing.file_path);
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }

    let next_active = state.db.get_active_model()?;
    if let Some(m) = next_active {
        sync_standby_for_active(state, &m).await;
    } else {
        state
            .sidecar
            .set_standby_model(None, None, "vulkan".to_string(), 4096, 99);
    }

    tracing::info!(model_id = %model_id, is_byom = existing.is_byom, "Deleted model entry");
    Ok(())
}

pub async fn sync_standby_for_active(state: &AppState, model: &InstalledModel) {
    if let Ok(cfg) = state.resolve_sidecar_config(model).await {
        state.sidecar.set_standby_model(
            Some(cfg.model_id),
            Some(cfg.model_display_name),
            cfg.backend,
            cfg.ctx_size,
            cfg.gpu_layers,
        );
    } else {
        state.sidecar.set_standby_model(
            Some(model.id.clone()),
            Some(model.display_name.clone()),
            "vulkan".to_string(),
            4096,
            99,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_byom_reference_never_deletes_user_disk_file() {
        let dir = std::env::temp_dir().join(format!("osmium_byom_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let gguf_file = dir.join("user_custom.gguf");
        fs::write(&gguf_file, b"GGUF_PLACEHOLDER").unwrap();

        let db = Database::open_in_memory().unwrap();
        let model = InstalledModel {
            id: "byom-1".to_string(),
            display_name: "User Custom".to_string(),
            tier: "custom_byom".to_string(),
            file_path: gguf_file.to_string_lossy().to_string(),
            file_size_bytes: 16,
            sha256: None,
            quantization: None,
            is_byom: true,
            block_count: Some(32),
            is_active: false,
            created_at: now_unix_ms(),
        };
        db.upsert_installed_model(&model).unwrap();
        let deleted = db.delete_installed_model("byom-1").unwrap().unwrap();
        assert!(deleted.is_byom);
        assert!(gguf_file.exists(), "BYOM file on disk must remain untouched");

        let _ = fs::remove_dir_all(&dir);
    }
}
