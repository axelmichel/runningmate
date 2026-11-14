use std::{fs, path::Path};

use tauri::State;

use crate::{
    activity_type::ViewMode,
    db::{
        Db,
        activities::{
            self, ActivityDetail, ActivityFilter, ActivityPage, ActivityQuery, ActivitySummary,
            ActivityUpdate,
        },
        media::{self, Media, MediaType},
        performances::{self, BestPerformance},
        weather::{self, Weather},
    },
    error::{AppError, AppResult},
    paths,
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

#[tauri::command(async)]
pub fn get_activity(db: State<'_, Db>, id: i64) -> AppResult<ActivityDetail> {
    activities::get(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn update_activity(db: State<'_, Db>, update: ActivityUpdate) -> AppResult<ActivityDetail> {
    let conn = db.conn()?;
    activities::update(&conn, &update)?;
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

#[tauri::command(async)]
pub fn list_media(db: State<'_, Db>, activity_id: i64) -> AppResult<Vec<Media>> {
    media::list(&*db.conn()?, activity_id)
}

/// Copies a photo or video into the media folder and attaches it to the activity. The Python
/// app moved the original file instead.
#[tauri::command(async)]
pub fn add_media(db: State<'_, Db>, activity_id: i64, source_path: String) -> AppResult<Media> {
    let source = Path::new(&source_path);
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    let media_type = MediaType::from_extension(extension)
        .ok_or_else(|| AppError::Invalid(format!("unsupported media file: {source_path}")))?;
    let file_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Invalid(format!("invalid file name: {source_path}")))?;

    let media_dir = paths::media_dir()?;
    fs::create_dir_all(&media_dir)?;
    let target = unique_path(&media_dir, file_name);
    fs::copy(source, &target)?;

    let conn = db.conn()?;
    let file_path = target.to_string_lossy().into_owned();
    let id = media::insert(&conn, activity_id, media_type, &file_path)?;
    Ok(Media {
        id,
        media_type,
        file_path,
    })
}

/// `dir/name`, or `dir/stem-N.ext` if that file already exists.
fn unique_path(dir: &Path, name: &str) -> std::path::PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("media");
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    (1..)
        .map(|n| dir.join(format!("{stem}-{n}.{extension}")))
        .find(|p| !p.exists())
        .expect("an unused file name exists")
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

#[cfg(test)]
mod tests {
    use super::unique_path;

    #[test]
    fn unique_path_avoids_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(dir.path(), "a.jpg"), dir.path().join("a.jpg"));
        std::fs::write(dir.path().join("a.jpg"), b"").unwrap();
        std::fs::write(dir.path().join("a-1.jpg"), b"").unwrap();
        assert_eq!(unique_path(dir.path(), "a.jpg"), dir.path().join("a-2.jpg"));
    }
}
