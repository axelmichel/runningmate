//! TCX import, re-import and folder sync. Imports run on a blocking thread and report each
//! file through `import://progress` events; weather is fetched in the background afterwards.

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};
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
/// Reported while files are read before importing; iCloud Drive may download them first.
pub const READING_EVENT: &str = "import://reading";

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReadingProgress {
    pub done: usize,
    pub total: usize,
    pub file: String,
}

/// Set by `cancel_import`; long-running imports check it between files and stop early.
#[derive(Default)]
pub struct ImportCancel(AtomicBool);

impl ImportCancel {
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    pub fn requested(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Asks the running import, folder sync or Garmin sync/upload to stop after the current file.
#[tauri::command]
pub fn cancel_import(cancel: State<'_, ImportCancel>) {
    cancel.0.store(true, Ordering::SeqCst);
}

fn cancelled(app: &AppHandle) -> bool {
    app.state::<ImportCancel>().requested()
}

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

/// Optional limits for a sync, as Unix timestamps (start of the first day, end of the last).
#[derive(Debug, Clone, Copy, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct DateRange {
    #[ts(type = "number", optional)]
    pub from: Option<i64>,
    #[ts(type = "number", optional)]
    pub to: Option<i64>,
}

impl DateRange {
    pub fn contains(&self, time: i64) -> bool {
        self.from.is_none_or(|from| time >= from) && self.to.is_none_or(|to| time <= to)
    }

    fn day(timestamp: i64) -> Option<String> {
        chrono::DateTime::from_timestamp(timestamp, 0).map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string()
        })
    }

    /// Local "YYYY-MM-DD" of the first day, for APIs that take dates.
    pub fn first_day(&self) -> Option<String> {
        self.from.and_then(Self::day)
    }

    pub fn last_day(&self) -> Option<String> {
        self.to.and_then(Self::day)
    }
}

/// Copies the files into a local staging folder, reporting each through `import://reading`.
/// Files in iCloud Drive may be downloaded on first access, which is the slow part of a folder
/// sync; afterwards everything is read from the local copies. Names are kept, as they are the
/// files' ids for duplicate detection.
fn stage(app: &AppHandle, files: &[PathBuf], staging: &Path) -> AppResult<Vec<PathBuf>> {
    std::fs::create_dir_all(staging)?;
    let mut staged = Vec::new();
    for (index, file) in files.iter().enumerate() {
        if cancelled(app) {
            break;
        }
        let name = file.file_name().map(|n| n.to_owned()).unwrap_or_default();
        let target = staging.join(&name);
        let _ = app.emit(
            READING_EVENT,
            ReadingProgress {
                done: index + 1,
                total: files.len(),
                file: name.to_string_lossy().into_owned(),
            },
        );
        // A file that can't be read is passed on as is, so the importer reports why.
        match std::fs::copy(file, &target) {
            Ok(_) => staged.push(target),
            Err(_) => staged.push(file.clone()),
        }
    }
    Ok(staged)
}

/// TCX files within the range, newest first (by their first trackpoint). Files whose start
/// can't be read are kept, at the end, so the importer can report what's wrong with them.
pub(crate) fn newest_first(files: Vec<PathBuf>, range: &DateRange) -> Vec<PathBuf> {
    let mut dated: Vec<(Option<i64>, PathBuf)> = files
        .into_iter()
        .map(|file| (importer::tcx_start(&file).map(|t| t.timestamp()), file))
        .filter(|(start, _)| start.is_none_or(|s| range.contains(s)))
        .collect();
    dated.sort_by_key(|(start, _)| std::cmp::Reverse(*start));
    dated.into_iter().map(|(_, file)| file).collect()
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportSummary {
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
    /// Stopped early by the user.
    pub cancelled: bool,
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
            ImportOutcome::Duplicate { .. } | ImportOutcome::Similar { .. } => self.duplicates += 1,
            ImportOutcome::Failed { .. } => self.failed += 1,
        }
    }
}

/// Imports files one by one; a failing file is reported and the rest continue.
fn import_paths(app: &AppHandle, files: &[PathBuf]) -> AppResult<ImportSummary> {
    let mut summary = ImportSummary::default();
    for (index, file) in files.iter().enumerate() {
        if cancelled(app) {
            summary.cancelled = true;
            break;
        }
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

/// Reads (stages) the files, orders them newest first within the range and imports them, all
/// on a blocking thread. The staging copies are removed afterwards.
async fn run_import(
    app: AppHandle,
    files: Vec<PathBuf>,
    range: DateRange,
) -> AppResult<ImportSummary> {
    app.state::<ImportCancel>().reset();
    tauri::async_runtime::spawn_blocking(move || {
        let staging = std::env::temp_dir()
            .join("runningmate-import")
            .join(chrono::Utc::now().timestamp_millis().to_string());
        let result = stage(&app, &files, &staging).and_then(|staged| {
            let mut summary = import_paths(&app, &newest_first(staged, &range))?;
            summary.cancelled |= cancelled(&app);
            Ok(summary)
        });
        let _ = std::fs::remove_dir_all(&staging);
        result
    })
    .await
    .map_err(|e| AppError::Invalid(format!("import stopped: {e}")))?
}

#[tauri::command]
pub async fn import_files(
    app: AppHandle,
    paths: Vec<String>,
    range: Option<DateRange>,
) -> AppResult<ImportSummary> {
    run_import(
        app,
        paths.into_iter().map(PathBuf::from).collect(),
        range.unwrap_or_default(),
    )
    .await
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
/// copied them into `imports` first), newest first and within the optional range. The sync
/// time is only recorded for a complete, unrestricted sync without failures, so nothing is
/// skipped.
#[tauri::command]
pub async fn sync_folder(app: AppHandle, range: Option<DateRange>) -> AppResult<ImportSummary> {
    let range = range.unwrap_or_default();
    let started = chrono::Local::now();
    let files = sync::status()?
        .new_files
        .into_iter()
        .map(PathBuf::from)
        .collect();
    let summary = run_import(app, files, range).await?;
    if summary.failed == 0 && !summary.cancelled && range.from.is_none() && range.to.is_none() {
        sync::mark_synced(started)?;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tcx(dir: &Path, name: &str, time: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(
            &path,
            format!("<TrainingCenterDatabase><Trackpoint><Time>{time}</Time></Trackpoint></TrainingCenterDatabase>"),
        )
        .unwrap();
        path
    }

    #[test]
    fn orders_newest_first_within_range() {
        let dir = tempfile::tempdir().unwrap();
        let old = tcx(dir.path(), "old.tcx", "2024-01-01T08:00:00Z");
        let new = tcx(dir.path(), "new.tcx", "2025-05-06T11:18:34Z");
        let mid = tcx(dir.path(), "mid.tcx", "2025-02-01T10:00:00Z");
        let broken = dir.path().join("broken.tcx");
        std::fs::write(&broken, "nothing").unwrap();
        let all = vec![old.clone(), broken.clone(), new.clone(), mid.clone()];

        assert_eq!(
            newest_first(all.clone(), &DateRange::default()),
            [new.clone(), mid.clone(), old, broken.clone()]
        );
        let from_2025 = DateRange {
            from: Some(1_735_689_600),
            to: None,
        };
        assert_eq!(
            newest_first(all.clone(), &from_2025),
            [new, mid.clone(), broken.clone()]
        );
        let only_feb = DateRange {
            from: Some(1_738_368_000),
            to: Some(1_740_787_199),
        };
        assert_eq!(newest_first(all, &only_feb), [mid, broken]);
    }
}
