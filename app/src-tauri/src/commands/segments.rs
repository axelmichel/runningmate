use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    activity_type::ViewMode,
    analysis,
    db::{
        Db,
        route_segments::{self, ActivitySegmentEffort, RouteSegment, SegmentDetail},
    },
    error::{AppError, AppResult},
    importer::{self, segment_match},
    paths,
};

/// Emitted with the segment id once the passes over a new segment have been found.
const UPDATED_EVENT: &str = "segments://updated";

/// Stores the stretch of the activity between `start_km` and `end_km` as a segment, then looks
/// for passes over it in every activity of the same sport in the background.
#[tauri::command(async)]
pub fn create_segment(
    app: AppHandle,
    db: State<'_, Db>,
    activity_id: i64,
    start_km: f64,
    end_km: f64,
    title: String,
) -> AppResult<SegmentDetail> {
    let imports_dir = paths::imports_dir()?;
    let detail = {
        let conn = db.conn()?;
        let track = analysis::load_track(&conn, activity_id, &imports_dir)?.ok_or_else(|| {
            AppError::NotFound(format!("archived TCX file for activity {activity_id}"))
        })?;
        let shape = segment_match::cut(&track, start_km, end_km).ok_or_else(|| {
            AppError::Invalid(format!(
                "a segment must be at least {} m long",
                segment_match::MIN_SEGMENT_LENGTH
            ))
        })?;
        let id = route_segments::create(
            &conn,
            &title,
            track.view,
            activity_id,
            &shape,
            chrono::Utc::now().timestamp(),
        )?;
        route_segments::get(&conn, id)?
    };
    let id = detail.segment.id;
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        if let Err(e) = importer::match_segment(&db, &imports_dir, id) {
            eprintln!("matching segment {id} failed: {e}");
        }
        let _ = app.emit(UPDATED_EVENT, id);
    });
    Ok(detail)
}

#[tauri::command(async)]
pub fn list_segments(
    db: State<'_, Db>,
    view: ViewMode,
    search: Option<String>,
) -> AppResult<Vec<RouteSegment>> {
    route_segments::list(&*db.conn()?, view, search.as_deref().unwrap_or(""))
}

#[tauri::command(async)]
pub fn get_segment(db: State<'_, Db>, id: i64) -> AppResult<SegmentDetail> {
    route_segments::get(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn rename_segment(db: State<'_, Db>, id: i64, title: String) -> AppResult<SegmentDetail> {
    let conn = db.conn()?;
    route_segments::rename(&conn, id, &title)?;
    route_segments::get(&conn, id)
}

#[tauri::command(async)]
pub fn delete_segment(db: State<'_, Db>, id: i64) -> AppResult<()> {
    route_segments::delete(&mut *db.conn()?, id)
}

#[tauri::command(async)]
pub fn activity_segment_efforts(
    db: State<'_, Db>,
    activity_id: i64,
) -> AppResult<Vec<ActivitySegmentEffort>> {
    route_segments::for_activity(&*db.conn()?, activity_id)
}
