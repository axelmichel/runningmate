use serde::{Serialize, Serializer};

/// Error type returned by every Tauri command.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("home directory could not be determined")]
    NoHomeDir,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// Commands send errors to the frontend as plain strings.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
