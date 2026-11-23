use crate::core::hardware::types::DiskSpaceInfo;
use crate::error::{AppError, AppResult};
use std::fs;
use std::path::{Path, PathBuf};

fn is_protected_windows_path(path: &Path) -> bool {
    let norm = path.to_string_lossy().to_lowercase().replace('/', "\\");
    let stripped = norm.strip_prefix("\\\\?\\").unwrap_or(&norm);

    let protected_prefixes = [
        "c:\\windows",
        "c:\\program files",
        "c:\\program files (x86)",
    ];

    for prefix in protected_prefixes {
        if stripped == prefix || stripped.starts_with(&format!("{prefix}\\")) {
            return true;
        }
    }
    false
}

pub fn validate_user_dir(dir_path: &Path) -> AppResult<PathBuf> {
    if dir_path.as_os_str().is_empty() || !dir_path.is_absolute() {
        return Err(AppError::InvalidPath(
            "Directory path must be a valid absolute path".to_string(),
        ));
    }

    if is_protected_windows_path(dir_path) {
        return Err(AppError::InvalidPath(
            "Windows system directories are not allowed for user storage".to_string(),
        ));
    }

    if !dir_path.exists() {
        fs::create_dir_all(dir_path)?;
    }
    if !dir_path.is_dir() {
        return Err(AppError::InvalidPath(
            "Target path is not a directory".to_string(),
        ));
    }

    let canonical = dunce::canonicalize(dir_path)?;
    if is_protected_windows_path(&canonical) {
        return Err(AppError::InvalidPath(
            "Windows system directories are not allowed".to_string(),
        ));
    }
    Ok(canonical)
}

pub fn validate_readable_file(file_path: &Path) -> AppResult<PathBuf> {
    if file_path.as_os_str().is_empty() || !file_path.is_absolute() {
        return Err(AppError::InvalidPath(
            "File path must be a valid absolute path".to_string(),
        ));
    }
    if !file_path.exists() || !file_path.is_file() {
        return Err(AppError::NotFound(format!(
            "File not found: {}",
            file_path.display()
        )));
    }
    let canonical = dunce::canonicalize(file_path)?;
    if is_protected_windows_path(&canonical) {
        return Err(AppError::InvalidPath(
            "Access to files inside Windows system directories is not allowed".to_string(),
        ));
    }
    Ok(canonical)
}

pub fn validate_export_file_path(file_path: &Path) -> AppResult<PathBuf> {
    if file_path.as_os_str().is_empty() || !file_path.is_absolute() {
        return Err(AppError::InvalidPath(
            "Export destination path must be a valid absolute path".to_string(),
        ));
    }
    if is_protected_windows_path(file_path) {
        return Err(AppError::InvalidPath(
            "Cannot save files inside Windows system directories".to_string(),
        ));
    }
    let parent = file_path.parent().ok_or_else(|| {
        AppError::InvalidPath("Export destination path has no parent directory".to_string())
    })?;
    let canonical_parent = validate_user_dir(parent)?;
    let file_name = file_path.file_name().ok_or_else(|| {
        AppError::InvalidPath("Invalid export filename".to_string())
    })?;
    Ok(canonical_parent.join(file_name))
}

#[cfg(windows)]
pub fn check_disk_space(dir_path: &Path) -> AppResult<DiskSpaceInfo> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let canonical = validate_user_dir(dir_path)?;
    let wide: Vec<u16> = canonical
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_bytes_available: u64 = 0;
    unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut free_bytes_available),
            None,
            None,
        )
        .map_err(|e| AppError::IoError(format!("GetDiskFreeSpaceExW: {e}")))?;
    }

    let probe_path = canonical.join(format!(".osmium_write_probe_{}", uuid::Uuid::new_v4()));
    let writable = match fs::File::create(&probe_path) {
        Ok(file) => {
            drop(file);
            let _ = fs::remove_file(&probe_path);
            true
        }
        Err(_) => false,
    };

    Ok(DiskSpaceInfo {
        free_bytes: free_bytes_available,
        writable,
    })
}

#[cfg(not(windows))]
pub fn check_disk_space(dir_path: &Path) -> AppResult<DiskSpaceInfo> {
    let _ = validate_user_dir(dir_path)?;
    Ok(DiskSpaceInfo {
        free_bytes: 0,
        writable: false,
    })
}
