use crate::core::hardware::disk;
use crate::db::models::AppSettings;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.db.get_settings()
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    let validated = validate_and_normalize_settings(settings)?;
    let previous = state.db.get_settings()?;

    let engine_settings_changed = previous.backend_override != validated.backend_override
        || previous.ctx_size_override != validated.ctx_size_override
        || previous.gpu_layers_override != validated.gpu_layers_override;

    if engine_settings_changed {
        state.coordinator.ensure_idle_for_admin_action().await?;
    }

    state.db.save_settings(&validated)?;

    if engine_settings_changed {
        let active_model = state.db.get_active_model()?;
        state.sidecar.stop(active_model.is_some()).await;
        if let Some(model) = active_model {
            if let Ok(cfg) = state.resolve_sidecar_config(&model).await {
                state.sidecar.set_standby_model(
                    Some(cfg.model_id),
                    Some(cfg.model_display_name),
                    cfg.backend,
                    cfg.ctx_size,
                    cfg.gpu_layers,
                );
            }
        }
    }

    Ok(validated)
}

fn validate_and_normalize_settings(mut settings: AppSettings) -> AppResult<AppSettings> {
    if !matches!(settings.display_mode.as_str(), "standard" | "compact") {
        return Err(AppError::InvalidInput(
            "display_mode must be 'standard' or 'compact'".to_string(),
        ));
    }
    if !matches!(settings.backend_override.as_str(), "auto" | "vulkan" | "cpu") {
        return Err(AppError::InvalidInput(
            "backend_override must be 'auto', 'vulkan', or 'cpu'".to_string(),
        ));
    }
    if !matches!(
        settings.ctx_size_override.as_str(),
        "auto" | "2048" | "4096" | "8192"
    ) {
        return Err(AppError::InvalidInput(
            "ctx_size_override must be 'auto', '2048', '4096', or '8192'".to_string(),
        ));
    }
    if settings.gpu_layers_override != "auto" {
        let parsed: u32 = settings.gpu_layers_override.parse().map_err(|_| {
            AppError::InvalidInput(
                "gpu_layers_override must be 'auto' or an integer between 0 and 200".to_string(),
            )
        })?;
        if parsed > 200 {
            return Err(AppError::InvalidInput(
                "gpu_layers_override maximum is 200".to_string(),
            ));
        }
    }
    if !matches!(settings.tts_provider.as_str(), "native" | "neural") {
        return Err(AppError::InvalidInput(
            "tts_provider must be 'native' or 'neural'".to_string(),
        ));
    }
    if !settings.tts_rate.is_finite() || !(0.5..=2.0).contains(&settings.tts_rate) {
        return Err(AppError::InvalidInput(
            "tts_rate must be between 0.5 and 2.0".to_string(),
        ));
    }
    let parsed_voice_prefs: serde_json::Value =
        serde_json::from_str(&settings.tts_voice_prefs_json).map_err(|_| {
            AppError::InvalidInput("tts_voice_prefs_json must be valid JSON".to_string())
        })?;
    if !parsed_voice_prefs.is_object() {
        return Err(AppError::InvalidInput(
            "tts_voice_prefs_json must be a JSON object".to_string(),
        ));
    }
    if settings.auto_idle_unload_minutes > 120 {
        return Err(AppError::InvalidInput(
            "auto_idle_unload_minutes maximum is 120".to_string(),
        ));
    }

    let model_dir = disk::validate_user_dir(&PathBuf::from(&settings.default_model_dir))?;
    let export_dir = disk::validate_user_dir(&PathBuf::from(&settings.default_export_dir))?;
    settings.default_model_dir = model_dir.to_string_lossy().to_string();
    settings.default_export_dir = export_dir.to_string_lossy().to_string();

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_validation_rejects_invalid_values_and_protected_dirs() {
        let temp_dir = std::env::temp_dir().join(format!("osmium_set_test_{}", uuid::Uuid::new_v4()));
        let valid = AppSettings {
            display_mode: "standard".to_string(),
            default_model_dir: temp_dir.join("models").to_string_lossy().to_string(),
            default_export_dir: temp_dir.join("exports").to_string_lossy().to_string(),
            backend_override: "auto".to_string(),
            ctx_size_override: "4096".to_string(),
            gpu_layers_override: "99".to_string(),
            tts_provider: "native".to_string(),
            tts_rate: 1.0,
            tts_voice_prefs_json: "{}".to_string(),
            auto_idle_unload_minutes: 10,
        };
        assert!(validate_and_normalize_settings(valid.clone()).is_ok());

        let bad_ctx = AppSettings {
            ctx_size_override: "99999".to_string(),
            ..valid.clone()
        };
        assert!(validate_and_normalize_settings(bad_ctx).is_err());

        let bad_tts_rate = AppSettings {
            tts_rate: 5.0,
            ..valid.clone()
        };
        assert!(validate_and_normalize_settings(bad_tts_rate).is_err());

        let bad_dir = AppSettings {
            default_model_dir: "C:\\Windows\\System32".to_string(),
            ..valid
        };
        assert!(validate_and_normalize_settings(bad_dir).is_err());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
