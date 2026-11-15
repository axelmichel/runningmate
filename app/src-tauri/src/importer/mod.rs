//! TCX import: parse, compute metrics and segments, store them and archive the file. Ported
//! from `importer/file/tcx_file.py`; charts and maps are no longer pre-rendered.

pub mod archive;
pub mod metrics;
pub mod outline;
pub mod route_image;
pub mod segment_match;
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
    db::{Db, activities, ai_summaries, performances, route_segments, tracks, users},
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
    /// An activity on the same day with about the same distance exists, e.g. the same run
    /// recorded by another app or device.
    Similar {
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
    pub track: Track,
}

/// Body weight (kg) from the profile, for the calorie and power estimates.
pub fn body_weight(conn: &Connection) -> AppResult<f64> {
    Ok(users::get(conn)?
        .and_then(|u| u.weight)
        .filter(|w| *w > 0.0)
        .unwrap_or(track::DEFAULT_BODY_WEIGHT))
}

/// Parses and computes everything for one TCX document; `weight` is the body weight in kg.
pub fn process(file_id: &str, xml: &str, weight: f64) -> AppResult<Processed> {
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
    let track = Track::new(activity.points, view, weight);
    Ok(Processed {
        file_id: file_id.to_string(),
        sport: activity.sport,
        view,
        metrics: metrics::compute(&track),
        segments: segments::split(&track),
        track,
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
    let processed = process(&file_id, &xml, body_weight(conn)?)?;
    let day = processed
        .metrics
        .start
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d")
        .to_string();
    if let Some(activity_id) = similar_activity(conn, &day, processed.metrics.distance)? {
        return Ok((ImportOutcome::Similar { activity_id }, None));
    }
    let activity_id = store(conn, &processed, None, language)?;
    archive::write(imports_dir, &file_id, &xml)?;
    Ok((
        ImportOutcome::Imported { activity_id },
        weather_point(activity_id, &processed.segments),
    ))
}

/// Distance tolerance for [`similar_activity`]: sources measure the same route slightly
/// differently, so ±1 % (at least 50 m) counts as the same distance.
fn same_distance(a: f64, b: f64) -> bool {
    (a - b).abs() <= (0.01 * a.max(b)).max(0.05)
}

/// An existing activity on the same local day ("YYYY-MM-DD") with about the same distance (km).
pub fn similar_activity(conn: &Connection, day: &str, distance: f64) -> AppResult<Option<i64>> {
    let mut stmt = conn.prepare(
        "SELECT id, distance FROM activities
         WHERE date(date, 'unixepoch', 'localtime') = ?1 AND distance IS NOT NULL",
    )?;
    let candidates: Vec<(i64, f64)> = stmt
        .query_map([day], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(candidates
        .into_iter()
        .find(|(_, d)| same_distance(*d, distance))
        .map(|(id, _)| id))
}

/// Start of a TCX file, read from its first `<Time>` without parsing the whole document.
pub fn tcx_start(path: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let xml = std::fs::read_to_string(path).ok()?;
    let start = xml.find("<Time>")? + "<Time>".len();
    let end = start + xml[start..].find("</Time>")?;
    chrono::DateTime::parse_from_rfc3339(xml[start..end].trim())
        .ok()
        .map(|t| t.to_utc())
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
    let processed = process(&file_id, &xml, body_weight(conn)?)?;
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

/// Runs `update` in its own transaction for every activity with an archived TCX, with the
/// activity recomputed from it. Activities whose file is missing or no longer parses are
/// skipped. Returns how many were updated.
///
/// The database is only locked per activity, so the app stays usable while this runs.
fn update_archived(
    db: &Db,
    imports_dir: &Path,
    update: impl Fn(&Transaction, i64, &Processed) -> AppResult<()>,
) -> AppResult<usize> {
    let (weight, activities) = {
        let conn = db.conn()?;
        let mut stmt =
            conn.prepare("SELECT id, file_id FROM activities WHERE file_id IS NOT NULL")?;
        let activities: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        (body_weight(&conn)?, activities)
    };
    let mut updated = 0;
    for (activity_id, file_id) in activities {
        let path = archive::archive_path(imports_dir, &file_id);
        let Ok(processed) = archive::read(&path).and_then(|xml| process(&file_id, &xml, weight))
        else {
            continue;
        };
        let mut conn = db.conn()?;
        let tx = conn.transaction()?;
        update(&tx, activity_id, &processed)?;
        tx.commit()?;
        updated += 1;
    }
    Ok(updated)
}

/// Finds the passes over a new route segment in every archived activity that comes near it.
/// Like [`update_archived`], the database is only locked per activity. Returns how many
/// activities pass over it.
pub fn match_segment(db: &Db, imports_dir: &Path, segment_id: i64) -> AppResult<usize> {
    let (view, shape, weight, candidates) = {
        let conn = db.conn()?;
        let (view, shape) = route_segments::shape(&conn, segment_id)?;
        let candidates = route_segments::candidate_activities(&conn, segment_id)?;
        (view, shape, body_weight(&conn)?, candidates)
    };
    let mut matched = 0;
    for (activity_id, file_id) in candidates {
        let path = archive::archive_path(imports_dir, &file_id);
        let Ok(activity) = archive::read(&path).and_then(|xml| tcx::parse(&xml)) else {
            continue;
        };
        if activity.points.len() < 2 {
            continue;
        }
        let efforts =
            segment_match::find_efforts(&shape, &Track::new(activity.points, view, weight));
        let mut conn = db.conn()?;
        let tx = conn.transaction()?;
        route_segments::store_segment_efforts(&tx, segment_id, activity_id, &efforts)?;
        tx.commit()?;
        matched += usize::from(!efforts.is_empty());
    }
    Ok(matched)
}

/// Recomputes calories and power (activity, per-type row and segments) of every activity with
/// an archived TCX, leaving everything else as it is, so edits survive.
pub fn recompute_energy(db: &Db, imports_dir: &Path) -> AppResult<usize> {
    update_archived(db, imports_dir, |tx, activity_id, processed| {
        let table = processed
            .view
            .table()
            .expect("process() rejects activities without a view");
        tx.execute(
            "UPDATE activities SET calories = ?1 WHERE id = ?2",
            params![processed.metrics.calories, activity_id],
        )?;
        tx.execute(
            &format!("UPDATE {table} SET avg_power = ?1 WHERE activity_id = ?2"),
            params![processed.metrics.avg_power, activity_id],
        )?;
        // `activity_details.activity_id` is a TEXT column in the Python schema.
        for (index, segment) in processed.segments.iter().enumerate() {
            tx.execute(
                "UPDATE activity_details SET seg_avg_power = ?1
                 WHERE activity_id = ?2 AND segment_id = ?3",
                params![segment.avg_power, activity_id.to_string(), index as i64],
            )?;
        }
        Ok(())
    })
}

/// Recomputes what pauses, standing still and the segment cuts got wrong before: calories,
/// total steps, the segments and the best segments. Title, distance, duration and the other
/// fields the edit page saves are left as they are.
pub fn recompute_pauses_and_segments(db: &Db, imports_dir: &Path) -> AppResult<usize> {
    update_archived(db, imports_dir, |tx, activity_id, processed| {
        tx.execute(
            "UPDATE activities SET calories = ?1 WHERE id = ?2",
            params![processed.metrics.calories, activity_id],
        )?;
        if processed.view != ViewMode::Cycling {
            let table = processed
                .view
                .table()
                .expect("process() rejects activities without a view");
            tx.execute(
                &format!("UPDATE {table} SET total_steps = ?1 WHERE activity_id = ?2"),
                params![processed.metrics.total_steps, activity_id],
            )?;
        }
        store_segments(tx, activity_id, &processed.segments)?;
        store_best_segments(tx, processed, activity_id)
    })
}

/// Re-imports every activity whose archived TCX has a clock jump (see `tcx::parse`), which put
/// it on the wrong day. Activities edited since are left alone, their date may be corrected by
/// hand. A title still generated from the wrong start is generated again. Returns where to
/// look the weather up again.
pub fn fix_clock_jumps(
    db: &Db,
    imports_dir: &Path,
    language: &str,
) -> AppResult<Vec<WeatherPoint>> {
    let activities: Vec<(i64, String, i64, String, String)> = {
        let conn = db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, file_id, date, title, activity_type FROM activities
             WHERE file_id IS NOT NULL AND edited = 0",
        )?;
        stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect::<Result<_, _>>()?
    };
    let mut weather = Vec::new();
    for (activity_id, file_id, date, title, activity_type) in activities {
        let path = archive::archive_path(imports_dir, &file_id);
        let Ok(activity) = archive::read(&path).and_then(|xml| tcx::parse(&xml)) else {
            continue;
        };
        if activity.clock_jumps == 0 {
            continue;
        }
        let view = ViewMode::from_activity_type(&activity_type);
        let old_title = chrono::DateTime::from_timestamp(date, 0)
            .map(|start| metrics::activity_title(view, start, language));
        let mut conn = db.conn()?;
        weather.extend(reimport(&mut conn, activity_id, imports_dir, language)?);
        if old_title.as_deref() == Some(title.as_str()) {
            let start: i64 = conn.query_row(
                "SELECT date FROM activities WHERE id = ?1",
                [activity_id],
                |row| row.get(0),
            )?;
            if let Some(start) = chrono::DateTime::from_timestamp(start, 0) {
                conn.execute(
                    "UPDATE activities SET title = ?1 WHERE id = ?2",
                    params![metrics::activity_title(view, start, language), activity_id],
                )?;
            }
        }
        // The old summary was written for the wrong day.
        ai_summaries::delete(&conn, activity_id)?;
    }
    Ok(weather)
}

fn data_fix_applied(db: &Db, name: &str) -> AppResult<bool> {
    Ok(db.conn()?.query_row(
        "SELECT EXISTS (SELECT 1 FROM data_fixes WHERE name = ?1)",
        [name],
        |row| row.get(0),
    )?)
}

fn mark_data_fix(db: &Db, name: &str) -> AppResult<()> {
    db.conn()?.execute(
        "INSERT OR IGNORE INTO data_fixes (name, applied) VALUES (?1, ?2)",
        params![name, chrono::Utc::now().timestamp()],
    )?;
    Ok(())
}

/// Corrects what earlier versions stored, each fix once per database: activities dated by a
/// clock jump ([`fix_clock_jumps`]), calories and power from the ACSM estimates
/// ([`recompute_energy`]), pauses and segment cuts ([`recompute_pauses_and_segments`]), then
/// best segments across a clock running backwards. Returns where to look the weather up again.
pub fn apply_data_fixes(
    db: &Db,
    imports_dir: &Path,
    language: &str,
) -> AppResult<Vec<WeatherPoint>> {
    let mut weather = Vec::new();
    if !data_fix_applied(db, "clock-jumps")? {
        weather = fix_clock_jumps(db, imports_dir, language)?;
        mark_data_fix(db, "clock-jumps")?;
    }
    if !data_fix_applied(db, "energy-acsm")? {
        recompute_energy(db, imports_dir)?;
        mark_data_fix(db, "energy-acsm")?;
    }
    if !data_fix_applied(db, "pauses-and-segments")? {
        recompute_pauses_and_segments(db, imports_dir)?;
        mark_data_fix(db, "pauses-and-segments")?;
    }
    // Best segments no longer span a segment whose clock ran backwards.
    if !data_fix_applied(db, "best-segments-clock")? {
        update_archived(db, imports_dir, |tx, activity_id, processed| {
            store_best_segments(tx, processed, activity_id)
        })?;
        mark_data_fix(db, "best-segments-clock")?;
    }
    Ok(weather)
}

/// Writes the activity, its per-type row, segments, best segments and passes over route
/// segments in one transaction.
pub(crate) fn store(
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
            let previous = activities::distance_of(&tx, id)?;
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
            activities::add_to_equipment(&tx, id, m.distance - previous)?;
            id
        }
    };
    store_type_row(&tx, p, activity_id)?;
    store_segments(&tx, activity_id, &p.segments)?;
    store_best_segments(&tx, p, activity_id)?;
    route_segments::store_efforts(&tx, activity_id, &p.track)?;
    tracks::store(&tx, activity_id, &tracks::encode_track(&p.track.points))?;
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
