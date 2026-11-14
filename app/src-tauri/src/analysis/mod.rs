//! Detail-window analyses that the Python app computed inside its widgets
//! (`ui/widget_trainings_effect.py`, `widget_fitness.py`, `widget_heart_rate_zones.py`,
//! `widget_heart_rate_tendency.py`) or rendered from re-parsed TCX data (charts and maps).

pub mod fitness;
pub mod health;
pub mod series;
pub mod training_effect;
pub mod zones;

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::{
    activity_type::ViewMode,
    error::{AppError, AppResult},
    importer::{archive, tcx, track::Track},
};

use chrono::{DateTime, NaiveDateTime};
use rusqlite::params_from_iter;
use serde::Serialize;
use ts_rs::TS;

use self::{
    fitness::FitnessDay,
    training_effect::{SegmentAverages, TrainingEffect},
    zones::ZoneTime,
};
use crate::db::{users, value::opt_f64};

/// Days of history used by the training effect baseline, fitness chart and heart-rate trend.
pub const HISTORY_DAYS: i64 = 90;

/// View and start time of an activity.
pub fn activity_basics(conn: &Connection, id: i64) -> AppResult<(ViewMode, i64)> {
    conn.query_row(
        "SELECT activity_type, date FROM activities WHERE id = ?1",
        [id],
        |row| {
            let activity_type: Option<String> = row.get(0)?;
            let date: Option<i64> = row.get(1)?;
            Ok((
                ViewMode::from_activity_type(&activity_type.unwrap_or_default()),
                date.unwrap_or(0),
            ))
        },
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("activity {id}")))
}

/// The activity's trackpoints with derived values, read from its archived TCX file.
pub fn load_track(conn: &Connection, id: i64, imports_dir: &Path) -> AppResult<Option<Track>> {
    let (view, _) = activity_basics(conn, id)?;
    let file_id: Option<String> = conn.query_row(
        "SELECT file_id FROM activities WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;
    let Some(file_id) = file_id else {
        return Ok(None);
    };
    let path = archive::archive_path(imports_dir, &file_id);
    if !path.exists() {
        return Ok(None);
    }
    let activity = tcx::parse(&archive::read(&path)?)?;
    Ok((activity.points.len() >= 2).then(|| Track::new(activity.points, view)))
}

const DAY_SECONDS: i64 = 24 * 3600;

/// Parses `seg_time_start`/`seg_time_end` ("2025-05-06 11:18:34+00:00").
fn segment_time(text: &str) -> Option<i64> {
    let trimmed = text.split('+').next()?.trim();
    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S%.f")
        .ok()
        .map(|t| t.and_utc().timestamp())
}

/// One row of the segments table.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SegmentRow {
    #[ts(type = "number")]
    pub index: i64,
    /// km.
    pub distance: Option<f64>,
    /// Seconds.
    #[ts(type = "number | null")]
    pub duration: Option<i64>,
    /// min/km.
    pub pace: Option<f64>,
    /// m/s.
    pub speed: Option<f64>,
    pub heart_rate: Option<f64>,
    pub power: Option<f64>,
    /// Steps per minute of one foot, as stored.
    pub steps: Option<f64>,
    /// Metres.
    pub elevation_gain: Option<f64>,
}

pub fn segments(conn: &Connection, id: i64) -> AppResult<Vec<SegmentRow>> {
    let mut stmt = conn.prepare(
        "SELECT segment_id, seg_distance, seg_time_start, seg_time_end, seg_avg_pace,
            seg_avg_speed, seg_avg_heart_rate, seg_avg_power, seg_avg_steps, seg_elevation_gain
         FROM activity_details WHERE activity_id = ?1 ORDER BY segment_id",
    )?;
    let rows = stmt.query_map([id.to_string()], |row| {
        let start: Option<String> = row.get("seg_time_start")?;
        let end: Option<String> = row.get("seg_time_end")?;
        let duration = match (
            start.as_deref().and_then(segment_time),
            end.as_deref().and_then(segment_time),
        ) {
            (Some(s), Some(e)) => Some(e - s),
            _ => None,
        };
        Ok(SegmentRow {
            index: row.get("segment_id")?,
            distance: opt_f64(row, "seg_distance")?,
            duration,
            pace: opt_f64(row, "seg_avg_pace")?,
            speed: opt_f64(row, "seg_avg_speed")?,
            heart_rate: opt_f64(row, "seg_avg_heart_rate")?,
            power: opt_f64(row, "seg_avg_power")?,
            steps: opt_f64(row, "seg_avg_steps")?,
            elevation_gain: opt_f64(row, "seg_elevation_gain")?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Heart-rate zone times; `None` until the user has saved zones.
pub fn heart_rate_zones(
    conn: &Connection,
    id: i64,
    imports_dir: &Path,
) -> AppResult<Option<Vec<ZoneTime>>> {
    let Some(bounds) = users::get(conn)?.and_then(|u| u.zones) else {
        return Ok(None);
    };
    if let Some(track) = load_track(conn, id, imports_dir)? {
        return Ok(Some(zones::from_track(&track, &bounds)));
    }
    let segments: Vec<(f64, f64)> = segments(conn, id)?
        .into_iter()
        .filter_map(|s| Some((s.heart_rate?, s.duration? as f64)))
        .collect();
    Ok(Some(zones::from_segments(&segments, &bounds)))
}

fn averages_from_row(row: &rusqlite::Row) -> rusqlite::Result<Option<SegmentAverages>> {
    Ok(
        match (
            opt_f64(row, "hr")?,
            opt_f64(row, "power")?,
            opt_f64(row, "speed")?,
            opt_f64(row, "pace")?,
        ) {
            (Some(heart_rate), Some(power), Some(speed), Some(pace)) => Some(SegmentAverages {
                heart_rate,
                power,
                speed,
                pace,
            }),
            _ => None,
        },
    )
}

pub fn training_effect(conn: &Connection, id: i64) -> AppResult<TrainingEffect> {
    let (view, date) = activity_basics(conn, id)?;
    let current = conn.query_row(
        "SELECT AVG(seg_avg_heart_rate) AS hr, AVG(seg_avg_power) AS power,
            AVG(seg_avg_speed) AS speed, AVG(seg_avg_pace) AS pace
         FROM activity_details WHERE activity_id = ?1",
        [id.to_string()],
        averages_from_row,
    )?;
    // Python only scores activities where every average is non-zero.
    let Some(current) = current.filter(|c| {
        [c.heart_rate, c.power, c.speed, c.pace]
            .iter()
            .all(|v| *v != 0.0)
    }) else {
        return Ok(TrainingEffect {
            score: 1,
            recovery_hours: training_effect::recovery_hours(1),
        });
    };

    let types = view.allowed_types();
    let placeholders = vec!["?"; types.len()].join(", ");
    let sql = format!(
        "SELECT seg_avg_heart_rate AS hr, seg_avg_power AS power, seg_avg_speed AS speed,
            seg_avg_pace AS pace
         FROM activity_details JOIN activities ON activity_details.activity_id = activities.id
         WHERE activities.id != ? AND activities.activity_type IN ({placeholders})
           AND activities.date BETWEEN ? AND ?"
    );
    let mut values: Vec<rusqlite::types::Value> = vec![id.into()];
    values.extend(types.iter().map(|t| (*t).to_string().into()));
    values.push((date - HISTORY_DAYS * DAY_SECONDS).into());
    values.push(date.into());
    let history: Vec<SegmentAverages> = conn
        .prepare(&sql)?
        .query_map(params_from_iter(values), averages_from_row)?
        .filter_map(|r| r.transpose())
        .collect::<Result<_, _>>()?;
    Ok(training_effect::training_effect(view, &current, &history))
}

/// Fitness, fatigue and form for the 90 days up to the activity.
pub fn fitness_trend(conn: &Connection, id: i64) -> AppResult<Vec<FitnessDay>> {
    let (_, date) = activity_basics(conn, id)?;
    fitness_between(conn, date - HISTORY_DAYS * DAY_SECONDS, date)
}

/// Fitness, fatigue and form for the training days between two Unix times. The curve starts
/// 42 days earlier so it has settled (Python extended its window in a similar way).
pub fn fitness_between(conn: &Connection, from: i64, to: i64) -> AppResult<Vec<FitnessDay>> {
    let mut stmt = conn.prepare(
        "SELECT date(activities.date, 'unixepoch') AS day, activities.activity_type AS type,
            AVG(seg_avg_heart_rate * seg_avg_pace / (seg_avg_speed + 1)) AS load
         FROM activity_details JOIN activities ON activity_details.activity_id = activities.id
         WHERE activities.date BETWEEN ?1 AND ?2
         GROUP BY activities.date, activities.activity_type
         ORDER BY activities.date",
    )?;
    let rows: Vec<(String, ViewMode, Option<f64>)> = stmt
        .query_map([from - 42 * DAY_SECONDS, to], |row| {
            let activity_type: Option<String> = row.get("type")?;
            Ok((
                row.get("day")?,
                ViewMode::from_activity_type(&activity_type.unwrap_or_default()),
                opt_f64(row, "load")?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let first_day = DateTime::from_timestamp(from, 0)
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    Ok(fitness::fitness_curve(&fitness::daily_loads(&rows))
        .into_iter()
        .filter(|d| d.day >= first_day)
        .collect())
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HeartRatePoint {
    /// Unix timestamp of the activity.
    #[ts(type = "number")]
    pub date: i64,
    pub avg_heart_rate: f64,
    /// Rolling mean over the last 30 activities, as in Python.
    pub trend: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HeartRateTrend {
    pub points: Vec<HeartRatePoint>,
    /// Zone 1, 3 and 5 bounds drawn as reference lines (min, ideal, max).
    #[ts(type = "[number, number, number] | null")]
    pub references: Option<[i64; 3]>,
}

/// Average heart rate of the same activity type over the 90 days up to the activity.
pub fn heart_rate_trend(conn: &Connection, id: i64) -> AppResult<HeartRateTrend> {
    let (view, date) = activity_basics(conn, id)?;
    let Some(table) = view.table() else {
        return Ok(HeartRateTrend {
            points: Vec::new(),
            references: None,
        });
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT {table}.avg_heart_rate AS hr, activities.date AS date FROM {table}
         JOIN activities ON activities.id = {table}.activity_id
         WHERE activities.date BETWEEN ?1 AND ?2 AND {table}.avg_heart_rate > 0
         ORDER BY activities.date"
    ))?;
    let raw: Vec<(i64, f64)> = stmt
        .query_map([date - HISTORY_DAYS * DAY_SECONDS, date], |row| {
            Ok((row.get("date")?, opt_f64(row, "hr")?.unwrap_or(0.0)))
        })?
        .collect::<Result<_, _>>()?;
    let window = (HISTORY_DAYS / 3) as usize;
    let points = raw
        .iter()
        .enumerate()
        .map(|(i, (date, hr))| {
            let from = (i + 1).saturating_sub(window);
            let slice = &raw[from..=i];
            HeartRatePoint {
                date: *date,
                avg_heart_rate: *hr,
                trend: slice.iter().map(|(_, h)| h).sum::<f64>() / slice.len() as f64,
            }
        })
        .collect();
    let references = users::get(conn)?
        .and_then(|u| u.zones)
        .map(|z| [z[0], z[2], z[4]]);
    Ok(HeartRateTrend { points, references })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    #[test]
    fn reads_segments_with_durations() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 1, "Running", 1_746_530_314, 2.0, "Run");
        conn.execute(
            "INSERT INTO activity_details (activity_id, segment_id, seg_distance, seg_time_start,
                seg_time_end, seg_avg_pace, seg_avg_heart_rate, seg_avg_power, seg_avg_speed)
             VALUES ('1', 0, 1.0, '2025-05-06 11:18:34+00:00', '2025-05-06 11:25:06+00:00',
                6.5, 150, 160, 2.5)",
            [],
        )
        .unwrap();
        let rows = segments(&conn, 1).unwrap();
        assert_eq!(rows[0].duration, Some(392));
        // Only segments of this activity; no zones until the user saved some.
        assert!(
            heart_rate_zones(&conn, 1, Path::new("/nonexistent"))
                .unwrap()
                .is_none()
        );
        let effect = training_effect(&conn, 1).unwrap();
        assert!((1..=12).contains(&effect.score));
        assert_eq!(heart_rate_trend(&conn, 1).unwrap().points.len(), 1);
        assert_eq!(fitness_trend(&conn, 1).unwrap().len(), 1);
    }
}
