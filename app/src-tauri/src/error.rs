use serde::{Serialize, Serializer};

/// Error type returned by every Tauri command.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("home directory could not be determined")]
    NoHomeDir,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("migration {version} failed: {source}")]
    Migration {
        version: i64,
        source: rusqlite::Error,
    },
    #[error("database connection is unusable after an earlier panic")]
    DbPoisoned,
    #[error("invalid settings file: {0}")]
    Settings(#[from] serde_json::Error),
    #[error("{0} not found")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
}

// Commands send errors to the frontend as plain strings.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
