use crate::{
    error::AppResult,
    settings::{self, SystemSettings},
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
