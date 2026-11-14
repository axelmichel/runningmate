//! TCX import, re-import and folder sync. Imports run on a blocking thread and report each
//! file through `import://progress` events; weather is fetched in the background afterwards.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use crate::{
    db::{Db, weather},
    error::{AppError, AppResult},
    importer::{self, ImportOutcome, WeatherPoint, outline::RouteOutline},
    open_meteo, paths, settings,
    sync::{self, FolderSyncStatus},
};

pub const PROGRESS_EVENT: &str = "import://progress";

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportProgress {
    /// Files handled so far, including this one.
    pub done: usize,
    pub total: usize,
    pub file: String,
    pub outcome: ImportOutcome,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportSummary {
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
}

/// Looks up and stores the weather for an imported activity without blocking the import.
fn fetch_weather_later(app: &AppHandle, point: WeatherPoint) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let Some(found) = open_meteo::fetch(point.latitude, point.longitude, &point.date).await
        else {
            return;
        };
        if let Ok(conn) = app.state::<Db>().conn() {
            let _ = weather::upsert(&conn, point.activity_id, &found);
        }
        let _ = app.emit("activity://updated", point.activity_id);
    });
}

/// Imports one file and starts its weather lookup; failures become a `Failed` outcome.
pub(crate) fn import_one(app: &AppHandle, file: &Path) -> AppResult<ImportOutcome> {
    let imports_dir = paths::imports_dir()?;
    let language = settings::load().map(|s| s.language).unwrap_or_default();
    let result = {
        let db = app.state::<Db>();
        let mut conn = db.conn()?;
        importer::import_file(&mut conn, file, &imports_dir, &language)
    };
    Ok(match result {
        Ok((outcome, weather_point)) => {
            if let Some(point) = weather_point {
                fetch_weather_later(app, point);
            }
            outcome
        }
        Err(e) => ImportOutcome::Failed {
            error: e.to_string(),
        },
    })
}

impl ImportSummary {
    pub(crate) fn count(&mut self, outcome: &ImportOutcome) {
        match outcome {
            ImportOutcome::Imported { .. } => self.imported += 1,
            ImportOutcome::Duplicate { .. } => self.duplicates += 1,
            ImportOutcome::Failed { .. } => self.failed += 1,
        }
    }
}

/// Imports files one by one; a failing file is reported and the rest continue.
fn import_paths(app: &AppHandle, files: &[PathBuf]) -> AppResult<ImportSummary> {
    let mut summary = ImportSummary::default();
    for (index, file) in files.iter().enumerate() {
        let outcome = import_one(app, file)?;
        summary.count(&outcome);
        let _ = app.emit(
            PROGRESS_EVENT,
            ImportProgress {
                done: index + 1,
                total: files.len(),
                file: file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                outcome,
            },
        );
    }
    Ok(summary)
}

async fn run_import(app: AppHandle, files: Vec<PathBuf>) -> AppResult<ImportSummary> {
    tauri::async_runtime::spawn_blocking(move || import_paths(&app, &files))
        .await
        .map_err(|e| AppError::Invalid(format!("import stopped: {e}")))?
}

#[tauri::command]
pub async fn import_files(app: AppHandle, paths: Vec<String>) -> AppResult<ImportSummary> {
    run_import(app, paths.into_iter().map(PathBuf::from).collect()).await
}

/// Recomputes an activity from its archived TCX file.
#[tauri::command(async)]
pub fn reimport_activity(app: AppHandle, db: State<'_, Db>, id: i64) -> AppResult<()> {
    let language = settings::load().map(|s| s.language).unwrap_or_default();
    let point = importer::reimport(&mut *db.conn()?, id, &paths::imports_dir()?, &language)?;
    if let Some(point) = point {
        fetch_weather_later(&app, point);
    }
    Ok(())
}

#[tauri::command(async)]
pub fn activity_route(db: State<'_, Db>, id: i64) -> AppResult<Option<RouteOutline>> {
    importer::route_outline(&*db.conn()?, id, &paths::imports_dir()?)
}

/// Saves the route as a PNG or SVG (chosen by the file extension) for sharing or photo overlays.
#[tauri::command(async)]
pub fn export_route_image(db: State<'_, Db>, id: i64, path: String) -> AppResult<()> {
    let outline = importer::route_outline(&*db.conn()?, id, &paths::imports_dir()?)?
        .ok_or_else(|| AppError::NotFound(format!("archived TCX file for activity {id}")))?;
    let path = PathBuf::from(path);
    let is_svg = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if is_svg {
        std::fs::write(&path, importer::route_image::svg(&outline))?;
    } else {
        std::fs::write(&path, importer::route_image::png(&outline)?)?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn folder_sync_status() -> AppResult<FolderSyncStatus> {
    sync::status()
}

#[tauri::command(async)]
pub fn set_sync_folder(folder: String) -> AppResult<FolderSyncStatus> {
    if !Path::new(&folder).is_dir() {
        return Err(AppError::Invalid(format!("not a folder: {folder}")));
    }
    sync::set_folder(&folder)
}

/// Imports the `.tcx` files changed since the last sync straight from the sync folder (Python
/// copied them into `imports` first) and records the sync time.
#[tauri::command]
pub async fn sync_folder(app: AppHandle) -> AppResult<ImportSummary> {
    let started = chrono::Local::now();
    let files = sync::status()?
        .new_files
        .into_iter()
        .map(PathBuf::from)
        .collect();
    let summary = run_import(app, files).await?;
    if summary.failed == 0 {
        sync::mark_synced(started)?;
    }
    Ok(summary)
}
