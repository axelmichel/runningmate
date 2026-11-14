mod activity_type;
mod commands;
mod db;
mod error;
mod paths;
mod settings;

use std::fs;

use tauri::Manager;

use crate::{db::Db, error::AppResult};

/// Creates the data folder if needed and opens (and migrates) the database.
fn open_database() -> AppResult<Db> {
    let data_dir = paths::data_dir()?;
    fs::create_dir_all(&data_dir)?;
    Db::open(&data_dir.join(db::DB_FILE_NAME))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(open_database()?);
            // Lets the webview load track images and media from the data folder.
            app.asset_protocol_scope()
                .allow_directory(paths::data_dir()?, true)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::activities::list_activities,
            commands::activities::activity_summary,
            commands::activities::activity_heatmap,
            commands::activities::get_activity,
            commands::activities::update_activity,
            commands::activities::set_activity_equipment,
            commands::activities::delete_activity,
            commands::activities::activity_type_details,
            commands::activities::list_media,
            commands::activities::add_media,
            commands::activities::delete_media,
            commands::activities::get_weather,
            commands::activities::activity_best_performances,
            commands::activities::top_best_performances,
            commands::users::get_user,
            commands::users::save_user,
            commands::users::save_heart_rate_zones,
            commands::users::suggested_heart_rate_zones,
            commands::users::list_equipment,
            commands::users::add_equipment,
            commands::users::set_equipment_in_use,
            commands::users::delete_equipment,
            commands::settings::get_settings,
            commands::settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::paths;

    #[test]
    fn data_dir_defaults_to_home() {
        if std::env::var_os(paths::DATA_DIR_ENV).is_some() {
            return;
        }
        let dir = paths::data_dir().unwrap();
        assert!(dir.ends_with("RunningData"));
        assert_eq!(dir.parent(), dirs::home_dir().as_deref());
    }
}
