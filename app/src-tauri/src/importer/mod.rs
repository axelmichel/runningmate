//! TCX import: parse, compute metrics and segments, store them and archive the file. Ported
//! from `importer/file/tcx_file.py`; charts and maps are no longer pre-rendered.

pub mod archive;
pub mod metrics;
pub mod outline;
pub mod segments;
pub mod tcx;
pub mod track;

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use ts_rs::TS;

use self::{metrics::ActivityMetrics, segments::Segment, track::Track};
use crate::{
    activity_type::ViewMode,
    db::performances,
    error::{AppError, AppResult},
};

/// Where the weather for an activity should be looked up: the middle segment.
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherPoint {
    pub activity_id: i64,
    pub latitude: f64,
    pub longitude: f64,
    /// UTC date of the segment start, "YYYY-MM-DD".
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "status")]
#[ts(export)]
pub enum ImportOutcome {
    Imported {
        #[ts(type = "number")]
        activity_id: i64,
    },
    /// A file with the same name was imported before.
    Duplicate {
        #[ts(type = "number")]
        activity_id: i64,
    },
    Failed {
        error: String,
    },
}

/// A parsed and computed activity, ready to store.
pub struct Processed {
    pub file_id: String,
    pub sport: String,
    pub view: ViewMode,
    pub metrics: ActivityMetrics,
    pub segments: Vec<Segment>,
}

/// Parses and computes everything for one TCX document.
pub fn process(file_id: &str, xml: &str) -> AppResult<Processed> {
    let activity = tcx::parse(xml)?;
    if activity.points.len() < 2 {
        return Err(AppError::Invalid(format!(
            "{file_id}: no GPS track in this file"
        )));
    }
    let view = ViewMode::from_activity_type(&activity.sport);
    if view == ViewMode::All {
        return Err(AppError::Invalid(format!(
            "{file_id}: unsupported sport \"{}\"",
            activity.sport
        )));
    }
    let track = Track::new(activity.points, view);
    Ok(Processed {
        file_id: file_id.to_string(),
        sport: activity.sport,
        view,
        metrics: metrics::compute(&track),
        segments: segments::split(&track),
    })
}

/// `file_id` for a TCX path: the file name without `.tcx`, as in Python.
pub fn file_id_for(path: &Path) -> AppResult<String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Invalid(format!("invalid file name: {}", path.display())))?;
    let lower = name.to_ascii_lowercase();
    if !lower.ends_with(".tcx") {
        return Err(AppError::Invalid(format!("not a .tcx file: {name}")));
    }
    Ok(name[..name.len() - 4].to_string())
}

fn activity_by_file_id(conn: &Connection, file_id: &str) -> AppResult<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM activities WHERE file_id = ?1",
            [file_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Imports one TCX file: stores the activity and archives the file in `imports_dir`. The
/// source file is left in place.
pub fn import_file(
    conn: &mut Connection,
    path: &Path,
    imports_dir: &Path,
    language: &str,
) -> AppResult<(ImportOutcome, Option<WeatherPoint>)> {
    let file_id = file_id_for(path)?;
    if let Some(activity_id) = activity_by_file_id(conn, &file_id)? {
        return Ok((ImportOutcome::Duplicate { activity_id }, None));
    }
    let xml = std::fs::read_to_string(path)?;
    let processed = process(&file_id, &xml)?;
    let activity_id = store(conn, &processed, None, language)?;
    archive::write(imports_dir, &file_id, &xml)?;
    Ok((
        ImportOutcome::Imported { activity_id },
        weather_point(activity_id, &processed.segments),
    ))
}

/// Recomputes an activity from its archived TCX. Title, comment, type detail and the assigned
/// shoe or bike are kept (the Python re-import overwrote the title).
pub fn reimport(
    conn: &mut Connection,
    activity_id: i64,
    imports_dir: &Path,
    language: &str,
) -> AppResult<Option<WeatherPoint>> {
    let file_id: Option<String> = conn
        .query_row(
            "SELECT file_id FROM activities WHERE id = ?1",
            [activity_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("activity {activity_id}")))?;
    let file_id = file_id.ok_or_else(|| {
        AppError::NotFound(format!("archived TCX file for activity {activity_id}"))
    })?;
    let xml = archive::read(&archive::archive_path(imports_dir, &file_id))?;
    let processed = process(&file_id, &xml)?;
    store(conn, &processed, Some(activity_id), language)?;
    Ok(weather_point(activity_id, &processed.segments))
}

/// Route outline of an activity from its archived TCX.
pub fn route_outline(
    conn: &Connection,
    activity_id: i64,
    imports_dir: &Path,
) -> AppResult<Option<outline::RouteOutline>> {
    let file_id: Option<String> = conn
        .query_row(
            "SELECT file_id FROM activities WHERE id = ?1",
            [activity_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let Some(file_id) = file_id else {
        return Ok(None);
    };
    let path = archive::archive_path(imports_dir, &file_id);
    if !path.exists() {
        return Ok(None);
    }
    Ok(outline::outline(
        &tcx::parse(&archive::read(&path)?)?.points,
    ))
}

fn weather_point(activity_id: i64, segments: &[Segment]) -> Option<WeatherPoint> {
    let middle = segments.get(segments.len() / 2)?;
    Some(WeatherPoint {
        activity_id,
        latitude: middle.latitude,
        longitude: middle.longitude,
        date: middle.time_start.format("%Y-%m-%d").to_string(),
    })
}

/// Writes the activity, its per-type row, segments and best segments in one transaction.
fn store(
    conn: &mut Connection,
    p: &Processed,
    existing: Option<i64>,
    language: &str,
) -> AppResult<i64> {
    let m = &p.metrics;
    let tx = conn.transaction()?;
    let activity_id = match existing {
        None => {
            tx.execute(
                "INSERT INTO activities (distance, activity_type, comment, title, duration, date,
                    calories, file_id, elevation_gain, edited, new)
                 VALUES (?1, ?2, '', ?3, ?4, ?5, ?6, ?7, ?8, 0, 1)",
                params![
                    m.distance,
                    p.sport,
                    metrics::activity_title(p.view, m.start, language),
                    m.duration,
                    m.start.timestamp(),
                    m.calories,
                    p.file_id,
                    m.elevation_gain
                ],
            )?;
            tx.last_insert_rowid()
        }
        Some(id) => {
            tx.execute(
                "UPDATE activities SET distance = ?1, activity_type = ?2, duration = ?3, date = ?4,
                    calories = ?5, elevation_gain = ?6
                 WHERE id = ?7",
                params![
                    m.distance,
                    p.sport,
                    m.duration,
                    m.start.timestamp(),
                    m.calories,
                    m.elevation_gain,
                    id
                ],
            )?;
            id
        }
    };
    store_type_row(&tx, p, activity_id)?;
    store_segments(&tx, activity_id, &p.segments)?;
    store_best_segments(&tx, p, activity_id)?;
    tx.commit()?;
    Ok(activity_id)
}

fn store_type_row(tx: &Transaction, p: &Processed, activity_id: i64) -> AppResult<()> {
    let m = &p.metrics;
    let table = p
        .view
        .table()
        .expect("process() rejects activities without a view");
    let has_steps = p.view != ViewMode::Cycling;
    let exists: bool = tx.query_row(
        &format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE activity_id = ?1)"),
        [activity_id],
        |row| row.get(0),
    )?;
    let paces = [
        metrics::format_min_sec(m.avg_pace),
        metrics::format_min_sec(m.fastest_pace),
        metrics::format_min_sec(m.slowest_pace),
        metrics::format_min_sec(m.pause_minutes),
    ];
    let steps = if has_steps {
        ", avg_steps = ?10, total_steps = ?11"
    } else {
        ""
    };
    if !exists {
        tx.execute(
            &format!(
                "INSERT INTO {table} (activity_id, elevation_gain, avg_speed) VALUES (?1, 0, 0)"
            ),
            [activity_id],
        )?;
    }
    let sql = format!(
        "UPDATE {table} SET elevation_gain = ?2, avg_speed = ?3, avg_power = ?4,
            avg_heart_rate = ?5, avg_pace = ?6, fastest_pace = ?7, slowest_pace = ?8,
            pause = ?9{steps}
         WHERE activity_id = ?1"
    );
    let mut values: Vec<rusqlite::types::Value> = vec![
        activity_id.into(),
        m.elevation_gain.into(),
        m.avg_speed.into(),
        m.avg_power.into(),
        m.avg_heart_rate.into(),
    ];
    values.extend(paces.into_iter().map(Into::into));
    if has_steps {
        values.push(m.avg_steps.into());
        values.push(m.total_steps.into());
    }
    tx.execute(&sql, rusqlite::params_from_iter(values))?;
    Ok(())
}

fn store_segments(tx: &Transaction, activity_id: i64, segments: &[Segment]) -> AppResult<()> {
    // `activity_details.activity_id` is a TEXT column in the Python schema.
    let id = activity_id.to_string();
    tx.execute("DELETE FROM activity_details WHERE activity_id = ?1", [&id])?;
    let mut insert = tx.prepare(
        "INSERT INTO activity_details (activity_id, segment_id, seg_latitude, seg_longitude,
            seg_avg_heart_rate, seg_avg_power, seg_avg_speed, seg_avg_pace, seg_avg_steps,
            seg_distance, seg_time_start, seg_time_end, seg_elevation_gain)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )?;
    for (index, s) in segments.iter().enumerate() {
        insert.execute(params![
            id,
            index as i64,
            s.latitude,
            s.longitude,
            s.avg_heart_rate,
            s.avg_power,
            s.avg_speed,
            s.avg_pace,
            s.avg_steps,
            s.distance,
            segments::format_time(s.time_start),
            segments::format_time(s.time_end),
            s.elevation_gain
        ])?;
    }
    Ok(())
}

/// Replaces the activity's rows in `best_performances` (Python only added rows, so re-imports
/// produced duplicates).
fn store_best_segments(tx: &Transaction, p: &Processed, activity_id: i64) -> AppResult<()> {
    tx.execute(
        "DELETE FROM best_performances WHERE activity_id = ?1",
        [activity_id],
    )?;
    let view_name = serde_json::to_value(p.view)?
        .as_str()
        .unwrap_or_default()
        .to_string();
    for best in segments::best_segments(&p.segments, performances::segment_distances(p.view)) {
        tx.execute(
            "INSERT INTO best_performances (activity_id, distance, best_time, date_time,
                activity_type)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                activity_id,
                best.distance,
                best.pace,
                segments::format_time(best.time_start),
                view_name
            ],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod parity_tests;
