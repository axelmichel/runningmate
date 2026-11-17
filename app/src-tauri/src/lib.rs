mod activity_type;
mod ai;
mod analysis;
mod commands;
mod db;
mod error;
mod garmin;
mod importer;
mod media_files;
mod menu;
mod nominatim;
mod open_meteo;
mod osm_routing;
mod paths;
mod photo_shrink;
mod photos_library;
mod photos_picker;
mod settings;
mod share;
mod storage;
mod sync;
mod training;

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
            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                commands::import::apply_data_fixes(&handle);
                // Place names still missing, e.g. of activities from before the lookup.
                commands::locations::locate_later(&handle);
            });
            app.manage(commands::garmin::PendingGarminSignIn::default());
            app.manage(commands::import::ImportCancel::default());
            app.manage(ai::AiQueue::default());
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
            commands::activities::route_heatmap,
            commands::share::map_tile,
            commands::share::read_image,
            commands::share::save_share_image,
            commands::share::share_image,
            commands::activities::get_activity,
            commands::activities::similar_activities,
            commands::activities::update_activity,
            commands::activities::set_activity_equipment,
            commands::activities::delete_activity,
            commands::activities::activity_type_details,
            commands::activities::list_media,
            commands::activities::recent_photos,
            commands::activities::gallery_media,
            commands::activities::add_media,
            commands::activities::add_media_from_photos,
            commands::activities::photos_access,
            commands::activities::request_photos_access,
            commands::activities::activity_photos,
            commands::activities::library_photo_thumbnail,
            commands::activities::add_library_photos,
            commands::activities::delete_media,
            commands::activities::photos_to_shrink,
            commands::activities::shrink_photos,
            commands::activities::get_weather,
            commands::activities::activity_best_performances,
            commands::activities::top_best_performances,
            commands::users::get_user,
            commands::users::save_user,
            commands::users::save_heart_rate_zones,
            commands::users::suggested_heart_rate_zones,
            commands::users::preview_heart_rate_zones,
            commands::users::list_equipment,
            commands::users::add_equipment,
            commands::users::set_equipment_in_use,
            commands::users::delete_equipment,
            commands::analysis::activity_series,
            commands::analysis::activity_segments,
            commands::analysis::activity_heart_rate_zones,
            commands::analysis::activity_effect_score,
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
            commands::repair::activity_fix_source,
            commands::repair::apply_activity_fix,
            commands::repair::apply_sample_fix,
            commands::repair::revert_activity_fix,
            commands::repair::pending_routes,
            commands::repair::pending_fix_source,
            commands::repair::apply_pending_fix,
            commands::repair::discard_pending_route,
            commands::repair::route_along_paths,
            commands::segments::create_segment,
            commands::segments::list_segments,
            commands::segments::get_segment,
            commands::segments::rename_segment,
            commands::segments::delete_segment,
            commands::segments::activity_segment_efforts,
            commands::goals::goal_progress,
            commands::goals::create_goal,
            commands::goals::update_goal,
            commands::goals::archive_goal,
            commands::goals::delete_goal,
            commands::goals::activity_goal_hits,
            commands::training::training_baseline,
            commands::training::preview_training_plan,
            commands::training::create_training_plan,
            commands::training::training_plans,
            commands::training::training_plan,
            commands::training::rename_training_plan,
            commands::training::adapt_training_plan,
            commands::training::archive_training_plan,
            commands::training::delete_training_plan,
            commands::training::update_training_session,
            commands::training::add_training_session,
            commands::training::delete_training_session,
            commands::training::skip_training_session,
            commands::training::link_training_session,
            commands::training::training_link_candidates,
            commands::training::training_sessions_between,
            commands::training::activity_training_session,
            commands::ai::ai_models,
            commands::ai::get_ai_summary,
            commands::ai::generate_ai_summary,
            commands::ai::get_training_note,
            commands::ai::generate_training_note,
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
