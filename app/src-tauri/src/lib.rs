mod activity_type;
mod analysis;
mod commands;
mod db;
mod error;
mod garmin;
mod importer;
mod media_files;
mod menu;
mod open_meteo;
mod paths;
mod photos_library;
mod photos_picker;
mod settings;
mod storage;
mod sync;

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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(open_database()?);
            app.manage(commands::garmin::PendingGarminSignIn::default());
            app.manage(commands::import::ImportCancel::default());
            menu::install(app)?;
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
            commands::activities::add_media_from_photos,
            commands::activities::photos_access,
            commands::activities::request_photos_access,
            commands::activities::activity_photos,
            commands::activities::library_photo_thumbnail,
            commands::activities::add_library_photos,
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
            commands::analysis::activity_series,
            commands::analysis::activity_segments,
            commands::analysis::activity_heart_rate_zones,
            commands::analysis::activity_training_effect,
            commands::analysis::fitness_trend,
            commands::analysis::heart_rate_trend,
            commands::analysis::health_report,
            commands::garmin::garmin_status,
            commands::garmin::garmin_sign_in,
            commands::garmin::garmin_submit_code,
            commands::garmin::garmin_sign_out,
            commands::garmin::garmin_sync,
            commands::garmin::garmin_export_candidates,
            commands::garmin::garmin_upload,
            commands::garmin::activity_links,
            commands::import::import_files,
            commands::import::cancel_import,
            commands::import::reimport_activity,
            commands::import::activity_route,
            commands::import::export_route_image,
            commands::import::folder_sync_status,
            commands::import::set_sync_folder,
            commands::import::sync_folder,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::storage_info,
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
