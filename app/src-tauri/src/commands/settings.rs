use crate::{
    error::AppResult,
    menu,
    settings::{self, SystemSettings},
    storage::{self, StorageInfo},
};

#[tauri::command(async)]
pub fn get_settings() -> AppResult<SystemSettings> {
    settings::load()
}

/// Saves the settings and rebuilds the native menu in the chosen language.
#[tauri::command(async)]
pub fn save_settings(app: tauri::AppHandle, settings: SystemSettings) -> AppResult<SystemSettings> {
    settings::save(&settings)?;
    // A menu that can't be rebuilt keeps its old language; the settings are still saved.
    let _ = menu::set_language(&app, &settings.language);
    if settings.location_lookup {
        super::locations::locate_later(&app);
    }
    Ok(settings)
}

#[tauri::command(async)]
pub fn storage_info() -> AppResult<StorageInfo> {
    storage::info()
}
