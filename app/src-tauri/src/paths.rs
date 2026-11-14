use std::path::PathBuf;

use crate::error::{AppError, AppResult};

/// Name of the data folder in the user's home directory, shared with the Python app.
const DATA_DIR_NAME: &str = "RunningData";

/// `~/RunningData`, where the database, imports, media and images live.
pub fn data_dir() -> AppResult<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(DATA_DIR_NAME))
        .ok_or(AppError::NoHomeDir)
}
