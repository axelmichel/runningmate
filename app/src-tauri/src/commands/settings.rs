use crate::{
    error::AppResult,
    settings::{self, SystemSettings},
    storage::{self, StorageInfo},
};

#[tauri::command(async)]
pub fn get_settings() -> AppResult<SystemSettings> {
    settings::load()
}

#[tauri::command(async)]
pub fn save_settings(settings: SystemSettings) -> AppResult<SystemSettings> {
    settings::save(&settings)?;
    Ok(settings)
}

#[tauri::command(async)]
pub fn storage_info() -> AppResult<StorageInfo> {
    storage::info()
}
