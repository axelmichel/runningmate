//! Files without any GPS (a treadmill run, a watch with GPS off), kept as
//! `imports/pending/{file_id}.tcx.tar.gz` until their route is drawn in the route fix editor.
//! Only then do they become an activity; the pending archive becomes its original.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::{
    ImportOutcome, WeatherPoint, activity_by_file_id, archive,
    repair::{self, FixEdit, FixSource},
    tcx::{self, TcxActivity},
};
use crate::error::{AppError, AppResult};

/// A file waiting for its route.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PendingRoute {
    pub file_id: String,
    pub sport: String,
    /// Unix seconds.
    #[ts(type = "number")]
    pub start: i64,
    /// Seconds.
    pub duration: f64,
    /// km as measured by the device (foot pod, treadmill), if it recorded any.
    pub distance: Option<f64>,
}

fn dir(imports_dir: &Path) -> PathBuf {
    imports_dir.join("pending")
}

/// The pending archive; `file_id` comes from the frontend, so it may not leave the folder.
fn path(imports_dir: &Path, file_id: &str) -> AppResult<PathBuf> {
    if file_id.is_empty() || file_id.contains(['/', '\\']) || file_id.starts_with('.') {
        return Err(AppError::Invalid(format!("invalid file id: {file_id}")));
    }
    Ok(archive::archive_path(&dir(imports_dir), file_id))
}

/// Whether the activity has times but too few positions to compute anything from.
pub fn needs_route(activity: &TcxActivity) -> bool {
    activity.points.len() < 2 && activity.samples.len() >= 2
}

pub fn keep(imports_dir: &Path, file_id: &str, xml: &str) -> AppResult<()> {
    path(imports_dir, file_id)?;
    archive::write(&dir(imports_dir), file_id, xml)?;
    Ok(())
}

fn read(imports_dir: &Path, file_id: &str) -> AppResult<TcxActivity> {
    let path = path(imports_dir, file_id)?;
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "file {file_id} waiting for a route"
        )));
    }
    tcx::parse(&archive::read(&path)?)
}

fn summary(file_id: String, activity: &TcxActivity) -> Option<PendingRoute> {
    let (first, last) = (activity.samples.first()?, activity.samples.last()?);
    Some(PendingRoute {
        file_id,
        sport: activity.sport.clone(),
        start: first.time.timestamp(),
        duration: (last.time - first.time).num_milliseconds() as f64 / 1000.0,
        distance: activity
            .samples
            .iter()
            .rev()
            .find_map(|s| s.distance)
            .map(|m| m / 1000.0),
    })
}

/// Files waiting for a route, newest first; unreadable ones are left out.
pub fn list(imports_dir: &Path) -> AppResult<Vec<PendingRoute>> {
    let Ok(entries) = std::fs::read_dir(dir(imports_dir)) else {
        return Ok(Vec::new());
    };
    let mut found: Vec<PendingRoute> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let file_id = name.strip_suffix(".tcx.tar.gz")?.to_string();
            let activity = tcx::parse(&archive::read(&entry.path()).ok()?).ok()?;
            summary(file_id, &activity)
        })
        .collect();
    found.sort_by_key(|p| std::cmp::Reverse(p.start));
    Ok(found)
}

/// Where the latest activity started, to open the map near home.
fn recent_position(conn: &Connection) -> AppResult<Option<(f64, f64)>> {
    Ok(conn
        .query_row(
            "SELECT d.seg_latitude, d.seg_longitude FROM activity_details d
             JOIN activities a ON d.activity_id = CAST(a.id AS TEXT)
             WHERE d.seg_latitude IS NOT NULL AND d.seg_longitude IS NOT NULL
             ORDER BY a.date DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

pub fn source(conn: &Connection, imports_dir: &Path, file_id: &str) -> AppResult<FixSource> {
    Ok(FixSource {
        center: recent_position(conn)?,
        ..repair::source(&read(imports_dir, file_id)?, false)
    })
}

/// Imports a pending file with its drawn route. When it becomes an activity (or one with its
/// name exists), the pending file moves to the activity's original archive, so the fix can be
/// reverted like any other; when a similar activity exists, it stays pending.
pub fn import(
    conn: &mut Connection,
    file_id: &str,
    edits: &[FixEdit],
    imports_dir: &Path,
    language: &str,
) -> AppResult<(ImportOutcome, Option<WeatherPoint>)> {
    let pending = path(imports_dir, file_id)?;
    if let Some(activity_id) = activity_by_file_id(conn, file_id)? {
        std::fs::remove_file(&pending)?;
        return Ok((ImportOutcome::Duplicate { activity_id }, None));
    }
    let fixed = repair::apply(&read(imports_dir, file_id)?, edits)?;
    let result = super::import_tcx(
        conn,
        file_id,
        &tcx::write(&fixed),
        &[],
        imports_dir,
        language,
    )?;
    if let (ImportOutcome::Imported { .. }, _) = result {
        std::fs::rename(&pending, archive::original_path(imports_dir, file_id))?;
    }
    Ok(result)
}

pub fn discard(imports_dir: &Path, file_id: &str) -> AppResult<()> {
    let path = path(imports_dir, file_id)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
