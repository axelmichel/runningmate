//! Route fix editor: samples and problems of an activity, applying drawn routes and reverting;
//! and files without GPS, which become activities once their route is drawn.

use tauri::{AppHandle, State};

use super::{ai, import::fetch_weather_later};
use crate::{
    db::{Db, ai_summaries},
    error::AppResult,
    importer::{
        self, ImportOutcome, WeatherPoint,
        pending::{self, PendingRoute},
        repair::{FixEdit, FixSource},
    },
    paths, settings,
};

#[tauri::command(async)]
pub fn activity_fix_source(db: State<'_, Db>, id: i64) -> AppResult<FixSource> {
    importer::fix_source(&*db.conn()?, id, &paths::imports_dir()?)
}

/// Like `reimport_activity` afterwards: the old summary and weather were for the old track.
fn recomputed(app: &AppHandle, db: &Db, id: i64, point: Option<WeatherPoint>) -> AppResult<()> {
    ai_summaries::delete(&*db.conn()?, id)?;
    if let Some(point) = point {
        fetch_weather_later(app, point);
    }
    ai::summarize_later(app, id);
    Ok(())
}

/// Replaces stretches of the track with drawn routes and recomputes the activity.
#[tauri::command(async)]
pub fn apply_activity_fix(
    app: AppHandle,
    db: State<'_, Db>,
    id: i64,
    edits: Vec<FixEdit>,
) -> AppResult<()> {
    let language = settings::load().map(|s| s.language).unwrap_or_default();
    let point = importer::fix_activity(
        &mut *db.conn()?,
        id,
        &edits,
        &paths::imports_dir()?,
        &language,
    )?;
    recomputed(&app, &db, id, point)
}

/// Resets a fixed activity to the file it was imported from.
#[tauri::command(async)]
pub fn revert_activity_fix(app: AppHandle, db: State<'_, Db>, id: i64) -> AppResult<()> {
    let language = settings::load().map(|s| s.language).unwrap_or_default();
    let point = importer::revert_fix(&mut *db.conn()?, id, &paths::imports_dir()?, &language)?;
    recomputed(&app, &db, id, point)
}

#[tauri::command(async)]
pub fn pending_routes() -> AppResult<Vec<PendingRoute>> {
    pending::list(&paths::imports_dir()?)
}

#[tauri::command(async)]
pub fn pending_fix_source(db: State<'_, Db>, file_id: String) -> AppResult<FixSource> {
    pending::source(&*db.conn()?, &paths::imports_dir()?, &file_id)
}

/// Imports a file without GPS with its drawn route.
#[tauri::command(async)]
pub fn apply_pending_fix(
    app: AppHandle,
    db: State<'_, Db>,
    file_id: String,
    edits: Vec<FixEdit>,
) -> AppResult<ImportOutcome> {
    let language = settings::load().map(|s| s.language).unwrap_or_default();
    let (outcome, point) = pending::import(
        &mut *db.conn()?,
        &file_id,
        &edits,
        &paths::imports_dir()?,
        &language,
    )?;
    if let Some(point) = point {
        fetch_weather_later(&app, point);
    }
    if let ImportOutcome::Imported { activity_id } = outcome {
        ai::summarize_later(&app, activity_id);
    }
    Ok(outcome)
}

#[tauri::command(async)]
pub fn discard_pending_route(file_id: String) -> AppResult<()> {
    pending::discard(&paths::imports_dir()?, &file_id)
}
