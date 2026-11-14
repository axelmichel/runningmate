use std::{fs, path::Path};

use tauri::{Manager, State};

use crate::{
    activity_type::ViewMode,
    db::{
        Db,
        activities::{
            self, ActivityDetail, ActivityFilter, ActivityPage, ActivityQuery, ActivitySummary,
            ActivityUpdate, DayTotal,
        },
        media::{self, ActivityPhoto, Media},
        performances::{self, BestPerformance},
        weather::{self, Weather},
    },
    error::{AppError, AppResult},
    media_files, paths,
    photos_library::{self, LibraryAsset, PhotosAccess},
    photos_picker,
};

#[tauri::command(async)]
pub fn list_activities(db: State<'_, Db>, query: ActivityQuery) -> AppResult<ActivityPage> {
    activities::list(&*db.conn()?, &query)
}

#[tauri::command(async)]
pub fn activity_summary(
    db: State<'_, Db>,
    view_mode: ViewMode,
    filter: Option<ActivityFilter>,
) -> AppResult<ActivitySummary> {
    activities::summary(&*db.conn()?, view_mode, &filter.unwrap_or_default())
}

/// Seconds in the 365-day heatmap window.
const HEATMAP_SPAN: i64 = 365 * 24 * 3600;

/// Daily totals for the activity heatmap. Like the Python heatmap, the window is the year up to
/// today, or the year after the filter's start date, or the year up to its end date.
#[tauri::command(async)]
pub fn activity_heatmap(
    db: State<'_, Db>,
    view_mode: ViewMode,
    filter: Option<ActivityFilter>,
) -> AppResult<Vec<DayTotal>> {
    let filter = filter.unwrap_or_default();
    let end = match (filter.min_date, filter.max_date) {
        (_, Some(max)) => max,
        (Some(min), None) => min + HEATMAP_SPAN,
        (None, None) => chrono::Local::now().timestamp(),
    };
    activities::daily_durations(&*db.conn()?, view_mode, &filter, end - HEATMAP_SPAN, end)
}

#[tauri::command(async)]
pub fn get_activity(db: State<'_, Db>, id: i64) -> AppResult<ActivityDetail> {
    activities::get(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn update_activity(db: State<'_, Db>, update: ActivityUpdate) -> AppResult<ActivityDetail> {
    let mut conn = db.conn()?;
    activities::update(&mut conn, &update)?;
    activities::get(&conn, update.id)
}

#[tauri::command(async)]
pub fn set_activity_equipment(
    db: State<'_, Db>,
    activity_id: i64,
    equipment_id: Option<i64>,
) -> AppResult<ActivityDetail> {
    let mut conn = db.conn()?;
    activities::set_equipment(&mut conn, activity_id, equipment_id)?;
    activities::get(&conn, activity_id)
}

#[tauri::command(async)]
pub fn delete_activity(db: State<'_, Db>, id: i64) -> AppResult<()> {
    activities::delete(&mut *db.conn()?, id)
}

#[tauri::command]
pub fn activity_type_details(view_mode: ViewMode) -> Vec<&'static str> {
    view_mode.type_details().to_vec()
}

/// Lets the webview load linked files, which live outside the data folder the asset protocol
/// is scoped to. Only the individual files are allowed, not their folders.
fn allow_linked(app: &tauri::AppHandle, media: &[Media]) {
    for item in media.iter().filter(|m| m.linked && !m.missing) {
        let _ = app.asset_protocol_scope().allow_file(&item.file_path);
    }
}

#[tauri::command(async)]
pub fn list_media(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    activity_id: i64,
) -> AppResult<Vec<Media>> {
    let media = media::list(&*db.conn()?, activity_id, &paths::media_dir()?)?;
    allow_linked(&app, &media);
    Ok(media)
}

/// The latest photos of all activities, for the dashboard.
#[tauri::command(async)]
pub fn recent_photos(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    limit: usize,
) -> AppResult<Vec<ActivityPhoto>> {
    let photos = media::recent_photos(&*db.conn()?, limit, &paths::media_dir()?)?;
    let media: Vec<Media> = photos.iter().map(|p| p.media.clone()).collect();
    allow_linked(&app, &media);
    Ok(photos)
}

/// Adds a photo or video to an activity: copied into the media folder, or with `link` the
/// original file is referenced (temporary and Photos-library files are always copied).
#[tauri::command(async)]
pub fn add_media(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    activity_id: i64,
    source_path: String,
    link: bool,
) -> AppResult<Media> {
    let media = media_files::add(
        &*db.conn()?,
        &paths::media_dir()?,
        activity_id,
        Path::new(&source_path),
        link,
    )?;
    allow_linked(&app, std::slice::from_ref(&media));
    Ok(media)
}

/// Shows Apple's photo picker and copies the picked photos and videos into the media folder.
/// Returns what was added (nothing when the picker was cancelled).
#[tauri::command]
pub async fn add_media_from_photos(
    window: tauri::WebviewWindow,
    activity_id: i64,
) -> AppResult<Vec<Media>> {
    let staging = std::env::temp_dir()
        .join("runningmate-photos")
        .join(chrono::Utc::now().timestamp_millis().to_string());
    let (sender, receiver) = tokio::sync::oneshot::channel();
    photos_picker::present(
        &window,
        staging.clone(),
        Box::new(move |files| {
            let _ = sender.send(files);
        }),
    )?;
    let files = receiver
        .await
        .map_err(|_| AppError::Invalid("the photo picker closed unexpectedly".into()))?;
    let media_dir = paths::media_dir()?;
    let added = {
        let db = window.state::<Db>();
        let conn = db.conn()?;
        files
            .iter()
            .map(|file| media_files::add(&conn, &media_dir, activity_id, file, false))
            .collect::<AppResult<Vec<_>>>()
    };
    let _ = fs::remove_dir_all(&staging);
    added
}

/// Removes the media row and its file. Files outside the media folder are left alone.
#[tauri::command(async)]
pub fn delete_media(db: State<'_, Db>, id: i64) -> AppResult<()> {
    let path = media::delete(&*db.conn()?, id)?;
    let media_dir = paths::media_dir()?;
    if let (Ok(file), Ok(dir)) = (fs::canonicalize(&path), fs::canonicalize(&media_dir))
        && file.starts_with(dir)
    {
        fs::remove_file(file)?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn get_weather(db: State<'_, Db>, activity_id: i64) -> AppResult<Option<Weather>> {
    weather::get(&*db.conn()?, activity_id)
}

#[tauri::command(async)]
pub fn activity_best_performances(
    db: State<'_, Db>,
    activity_id: i64,
) -> AppResult<Vec<BestPerformance>> {
    performances::for_activity(&*db.conn()?, activity_id)
}

#[tauri::command(async)]
pub fn top_best_performances(
    db: State<'_, Db>,
    view_mode: ViewMode,
) -> AppResult<Vec<BestPerformance>> {
    performances::top(&*db.conn()?, view_mode)
}

/// Minutes before the start and after the end of an activity that count as "during" it, for
/// photos taken just before setting off or right after finishing.
const PHOTO_MARGIN_SECS: i64 = 10 * 60;

#[tauri::command(async)]
pub fn photos_access() -> PhotosAccess {
    photos_library::access()
}

/// Asks for Photos library access (the system dialog appears only the first time).
#[tauri::command]
pub async fn request_photos_access() -> AppResult<PhotosAccess> {
    tauri::async_runtime::spawn_blocking(photos_library::request_access)
        .await
        .map_err(|e| AppError::Invalid(e.to_string()))
}

/// Photos and videos in the Photos library taken during the activity (± a few minutes).
#[tauri::command(async)]
pub fn activity_photos(db: State<'_, Db>, activity_id: i64) -> AppResult<Vec<LibraryAsset>> {
    let (detail, media) = {
        let conn = db.conn()?;
        let detail = activities::get(&conn, activity_id)?;
        let media = media::list(&conn, activity_id, &paths::media_dir()?)?;
        (detail, media)
    };
    let Some(start) = detail.date else {
        return Ok(Vec::new());
    };
    let end = start + detail.duration.unwrap_or(0);
    let mut assets =
        photos_library::assets_between(start - PHOTO_MARGIN_SECS, end + PHOTO_MARGIN_SECS)?;
    for asset in &mut assets {
        asset.added =
            photos_library::is_added(&asset.id, media.iter().map(|m| m.file_path.as_str()));
    }
    Ok(assets)
}

/// Path of a small preview of a Photos library item (inside the data folder, so the webview
/// may load it).
#[tauri::command]
pub async fn library_photo_thumbnail(id: String) -> AppResult<String> {
    let cache = paths::data_dir()?.join("cache").join("photos");
    tauri::async_runtime::spawn_blocking(move || photos_library::thumbnail(&id, &cache))
        .await
        .map_err(|e| AppError::Invalid(e.to_string()))?
        .map(|path| path.to_string_lossy().into_owned())
}

/// Copies the chosen Photos library items into the media folder.
#[tauri::command]
pub async fn add_library_photos(
    app: tauri::AppHandle,
    activity_id: i64,
    ids: Vec<String>,
) -> AppResult<Vec<Media>> {
    let staging = std::env::temp_dir()
        .join("runningmate-photos")
        .join(chrono::Utc::now().timestamp_millis().to_string());
    let exported = {
        let staging = staging.clone();
        tauri::async_runtime::spawn_blocking(move || {
            ids.iter()
                .map(|id| photos_library::export(id, &staging))
                .collect::<AppResult<Vec<_>>>()
        })
        .await
        .map_err(|e| AppError::Invalid(e.to_string()))?
    };
    let added = exported.and_then(|files| {
        let media_dir = paths::media_dir()?;
        let db = app.state::<Db>();
        let conn = db.conn()?;
        files
            .iter()
            .map(|file| media_files::add(&conn, &media_dir, activity_id, file, false))
            .collect::<AppResult<Vec<_>>>()
    });
    let _ = fs::remove_dir_all(&staging);
    added
}
