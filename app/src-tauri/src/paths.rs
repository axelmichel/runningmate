use std::path::PathBuf;

use crate::error::{AppError, AppResult};

/// Name of the data folder in the user's home directory, shared with the Python app.
const DATA_DIR_NAME: &str = "RunningData";

/// Overrides the data folder, e.g. to develop against a copy of the real data.
pub const DATA_DIR_ENV: &str = "RUNNINGMATE_DATA_DIR";

/// `~/RunningData` (or `$RUNNINGMATE_DATA_DIR`), where the database, imports, media and images
/// live.
pub fn data_dir() -> AppResult<PathBuf> {
    if let Some(dir) = std::env::var_os(DATA_DIR_ENV).filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    dirs::home_dir()
        .map(|home| home.join(DATA_DIR_NAME))
        .ok_or(AppError::NoHomeDir)
}

/// `settings.json` and `sync.json` live here.
pub fn appdata_dir() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("appdata"))
}

/// Uploaded photos and videos.
pub fn media_dir() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("media"))
}

/// Archived `{file_id}.tcx.tar.gz` files.
pub fn imports_dir() -> AppResult<PathBuf> {
    Ok(data_dir()?.join("imports"))
}
