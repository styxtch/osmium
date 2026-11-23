use std::fs;
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

const MAX_LOG_FILES: usize = 5;

/// Initializes daily rotating privacy-safe diagnostic logging in `log_dir`.
///
/// PRIVACY RULE: Never log `source_text`, `translated_text`, subtitle dialogue,
/// glossary terms, or LLM prompts anywhere in the application.
pub fn init_logging(log_dir: &Path) -> Option<WorkerGuard> {
    if let Err(err) = fs::create_dir_all(log_dir) {
        eprintln!("Failed to create log directory {}: {err}", log_dir.display());
        return None;
    }

    prune_old_logs(log_dir, MAX_LOG_FILES);

    let file_appender = tracing_appender::rolling::daily(log_dir, "osmium.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,osmium_lib=debug"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .try_init();

    Some(guard)
}

fn prune_old_logs(log_dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };

    let mut log_files: Vec<_> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy();
            if !name.starts_with("osmium.log") {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();

    if log_files.len() <= keep {
        return;
    }

    // Oldest first so we can remove excess files while retaining the `keep` newest.
    log_files.sort_by_key(|(modified, _)| *modified);
    let remove_count = log_files.len().saturating_sub(keep);
    for (_, path) in log_files.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
}
