use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("GPU memory is full due to other applications. Close heavy apps or switch to Qwen 3 4B (VRAM Saver).")]
    OutOfMemory,

    #[error("GPU does not support Vulkan. Osmium is running in CPU mode.")]
    VulkanInitFailed,

    #[error("Local model file was not found at its previous location: {0}")]
    ByomFileMissing(String),

    #[error("Subtitle Translator is currently processing an active batch. Pause or wait until it finishes.")]
    EngineBusySubtitleJob,

    #[error("Local AI engine stopped unexpectedly: {0}")]
    SidecarCrashed(String),

    #[error("No active AI model selected. Download or import a model in Model Manager.")]
    NoActiveModel,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid or restricted path: {0}")]
    InvalidPath(String),

    #[error("Insufficient disk space: {0}")]
    DiskSpaceInsufficient(String),

    #[error("SHA-256 checksum verification failed: {0}")]
    Sha256Mismatch(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Local database error: {0}")]
    DbError(String),

    #[error("File I/O error: {0}")]
    IoError(String),

    #[error("HTTP communication error: {0}")]
    HttpError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::OutOfMemory => "OUT_OF_MEMORY",
            Self::VulkanInitFailed => "VULKAN_INIT_FAILED",
            Self::ByomFileMissing(_) => "BYOM_FILE_MISSING",
            Self::EngineBusySubtitleJob => "ENGINE_BUSY_SUBTITLE_JOB",
            Self::SidecarCrashed(_) => "SIDECAR_CRASHED",
            Self::NoActiveModel => "NO_ACTIVE_MODEL",
            Self::InvalidInput(_) => "INVALID_INPUT",
            Self::InvalidPath(_) => "INVALID_PATH",
            Self::DiskSpaceInsufficient(_) => "DISK_SPACE_INSUFFICIENT",
            Self::Sha256Mismatch(_) => "SHA256_MISMATCH",
            Self::NotFound(_) => "NOT_FOUND",
            Self::DbError(_) => "DB_ERROR",
            Self::IoError(_) => "IO_ERROR",
            Self::HttpError(_) => "HTTP_ERROR",
            Self::InternalError(_) => "INTERNAL_ERROR",
        }
    }
}

#[derive(Serialize)]
struct SerializedAppError {
    code: &'static str,
    message: String,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializedAppError {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        Self::DbError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        use std::error::Error;
        let mut details = err.to_string();
        let mut src = err.source();
        while let Some(cause) = src {
            details.push_str(": ");
            details.push_str(&cause.to_string());
            src = cause.source();
        }
        Self::HttpError(details)
    }
}

pub type AppResult<T> = Result<T, AppError>;
