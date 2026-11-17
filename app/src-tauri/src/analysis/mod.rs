//! Detail-window analyses that the Python app computed inside its widgets
//! (`ui/widget_trainings_effect.py`, `widget_fitness.py`, `widget_heart_rate_zones.py`,
//! `widget_heart_rate_tendency.py`) or rendered from re-parsed TCX data (charts and maps).

pub mod cardio;
pub mod comparison;
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
    importer::{self, archive, tcx, track::Track},
};

use chrono::{DateTime, NaiveDateTime};
use serde::Serialize;
use ts_rs::TS;

use self::{
    fitness::FitnessDay,
    training_effect::{HeartRateBasis, TrainingEffect},
    zones::ZoneTime,
};
use crate::db::{users, value::opt_f64};

/// Days of history used by the fitness chart, the heart-rate trend and the VO2max of the
/// training effect.
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
    let weight = importer::body_weight(conn)?;
    Ok((activity.points.len() >= 2).then(|| Track::new(activity.points, view, weight)))
}

const DAY_SECONDS: i64 = 24 * 3600;

/// Parses `seg_time_start`/`seg_time_end` ("2025-05-06 11:18:34+00:00").
fn segment_time(text: &str) -> Option<i64> {
    segment_millis(text).map(|ms| ms.div_euclid(1000))
}

fn segment_millis(text: &str) -> Option<i64> {
    let trimmed = text.split('+').next()?.trim();
    NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S%.f")
        .ok()
        .map(|t| t.and_utc().timestamp_millis())
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
    /// Grade-adjusted pace, min/km; `None` for rides and splits stored before it was computed.
    pub gap: Option<f64>,
    /// m/s.
    pub speed: Option<f64>,
    pub heart_rate: Option<f64>,
    pub power: Option<f64>,
    /// Steps per minute of one foot, as stored.
    pub steps: Option<f64>,
    /// Metres.
    pub elevation_gain: Option<f64>,
    /// Unix milliseconds of the first and last sample, for correcting the split's samples.
    #[ts(type = "number | null")]
    pub start: Option<i64>,
    #[ts(type = "number | null")]
    pub end: Option<i64>,
}

pub fn segments(conn: &Connection, id: i64) -> AppResult<Vec<SegmentRow>> {
    let mut stmt = conn.prepare(
        "SELECT segment_id, seg_distance, seg_time_start, seg_time_end, seg_avg_pace,
            seg_avg_speed, seg_avg_heart_rate, seg_avg_power, seg_avg_steps, seg_elevation_gain,
            seg_avg_gap
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
            gap: opt_f64(row, "seg_avg_gap")?,
            speed: opt_f64(row, "seg_avg_speed")?,
            heart_rate: opt_f64(row, "seg_avg_heart_rate")?,
            power: opt_f64(row, "seg_avg_power")?,
            steps: opt_f64(row, "seg_avg_steps")?,
            elevation_gain: opt_f64(row, "seg_elevation_gain")?,
            start: start.as_deref().and_then(segment_millis),
            end: end.as_deref().and_then(segment_millis),
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Heart-rate zone times; `None` while the user's zone method has no zones.
pub fn heart_rate_zones(
    conn: &Connection,
    id: i64,
    imports_dir: &Path,
) -> AppResult<Option<Vec<ZoneTime>>> {
    let Some(bounds) = users::zones(conn)? else {
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

/// Training effect from the heart rate of the archived track or, without one, of the splits;
/// `None` without any heart rate. Computed on demand like the zones: only the detail window and
/// the AI summary ask for it, one activity at a time.
pub fn training_effect(
    conn: &Connection,
    id: i64,
    imports_dir: &Path,
) -> AppResult<Option<TrainingEffect>> {
    let (_, date) = activity_basics(conn, id)?;
    let user = users::get(conn)?;
    let day = DateTime::from_timestamp(date, 0).map(|t| t.date_naive());
    let basis = match &user {
        Some(u) => HeartRateBasis::resolve(
            u.hr_max,
            u.zones.map(|z| z.upper[4]).or(u.custom_zones.map(|z| z[4])),
            u.birthday
                .as_deref()
                .zip(day)
                .and_then(|(birthday, day)| users::age_on(birthday, day))
                .or(u.age),
            u.hr_min,
        ),
        None => HeartRateBasis::resolve(None, None, None, None),
    };
    let estimate = cardio::estimate(health::run_vo2max(
        conn,
        date - HISTORY_DAYS * DAY_SECONDS,
        date,
    )?)
    .map(|v| (v, health::Vo2maxSource::Runs));
    let vo2max = estimate.or_else(|| {
        user.as_ref()
            .and_then(|u| u.vo2max)
            .filter(|v| *v > 0.0)
            .map(|v| (v, health::Vo2maxSource::Profile))
    });
    if let Some(track) = load_track(conn, id, imports_dir)?
        && let Some(effect) = training_effect::evaluate(
            &training_effect::track_samples(&track),
            &basis,
            vo2max,
            false,
        )
    {
        return Ok(Some(effect));
    }
    let splits: Vec<(f64, f64)> = segments(conn, id)?
        .into_iter()
        .filter_map(|s| Some((s.heart_rate?, s.duration? as f64)))
        .collect();
    Ok(training_effect::evaluate(
        &training_effect::split_samples(&splits),
        &basis,
        vo2max,
        true,
    ))
}

/// Fitness, fatigue and form for the 90 days up to the activity.
pub fn fitness_trend(conn: &Connection, id: i64) -> AppResult<Vec<FitnessDay>> {
    let (_, date) = activity_basics(conn, id)?;
    fitness_between(conn, date - HISTORY_DAYS * DAY_SECONDS, date)
}

/// Fitness, fatigue and form for every training day up to `to`. The curve always starts at the
/// first activity: it only moves on training days, so a curve started shortly before a window
/// would not have settled, and the same day would show different values for different windows.
/// Segments without heart rate or pace have no load and are left out; an activity recorded
/// without heart rate used to count as a training day with load 0 and pull the curve down.
pub fn fitness_until(conn: &Connection, to: i64) -> AppResult<Vec<FitnessDay>> {
    let mut stmt = conn.prepare(
        "SELECT date(activities.date, 'unixepoch') AS day, activities.activity_type AS type,
            AVG(seg_avg_heart_rate * seg_avg_pace / (seg_avg_speed + 1)) AS load
         FROM activity_details JOIN activities ON activity_details.activity_id = activities.id
         WHERE activities.date <= ?1 AND seg_avg_heart_rate > 0 AND seg_avg_pace > 0
         GROUP BY activities.date, activities.activity_type
         ORDER BY activities.date",
    )?;
    let rows: Vec<(String, ViewMode, Option<f64>)> = stmt
        .query_map([to], |row| {
            let activity_type: Option<String> = row.get("type")?;
            Ok((
                row.get("day")?,
                ViewMode::from_activity_type(&activity_type.unwrap_or_default()),
                opt_f64(row, "load")?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(fitness::fitness_curve(&fitness::daily_loads(&rows)))
}

/// The training days of [`fitness_until`] from `from` on.
pub fn fitness_between(conn: &Connection, from: i64, to: i64) -> AppResult<Vec<FitnessDay>> {
    Ok(in_period(fitness_until(conn, to)?, from))
}

/// The days of a fitness curve from `from` (Unix time) on.
pub fn in_period(curve: Vec<FitnessDay>, from: i64) -> Vec<FitnessDay> {
    let first_day = DateTime::from_timestamp(from, 0)
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    curve.into_iter().filter(|d| d.day >= first_day).collect()
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
    let references = users::zones(conn)?.map(|z| [z.upper[0], z.upper[2], z.upper[4]]);
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
        // Without a profile and a track: the default heart rates and the split averages.
        let effect = training_effect(&conn, 1, Path::new("/nonexistent"))
            .unwrap()
            .unwrap();
        assert!(effect.from_splits);
        assert_eq!(
            effect.max_heart_rate_source,
            training_effect::MaxHeartRateSource::Default
        );
        assert!(effect.aerobic > 0.0 && effect.aerobic < 2.0, "{effect:?}");
        assert_eq!(effect.anaerobic, 0.0);
        assert_eq!(effect.vo2max, None);

        // The profile's heart rates count: closer to the maximum is harder.
        conn.execute(
            "INSERT INTO users (name, hr_min, hr_max, vo2max) VALUES ('Me', 50, 160, 40)",
            [],
        )
        .unwrap();
        let profile = training_effect(&conn, 1, Path::new("/nonexistent"))
            .unwrap()
            .unwrap();
        assert_eq!(
            (profile.resting_heart_rate, profile.max_heart_rate),
            (50, 160)
        );
        assert_eq!(profile.vo2max_source, Some(health::Vo2maxSource::Profile));
        assert!(profile.peak_epoc > effect.peak_epoc);
        assert_eq!(heart_rate_trend(&conn, 1).unwrap().points.len(), 1);
        assert_eq!(fitness_trend(&conn, 1).unwrap().len(), 1);
    }

    #[test]
    fn history_without_heart_rate_has_no_load() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let day = 1_746_530_314;
        let segment = |id: i64, hr: i64| {
            conn.execute(
                "INSERT INTO activity_details (activity_id, segment_id, seg_distance,
                    seg_avg_pace, seg_avg_heart_rate, seg_avg_power, seg_avg_speed)
                 VALUES (?1, 0, 1.0, 6.0, ?2, 160, 2.8)",
                rusqlite::params![id.to_string(), hr],
            )
            .unwrap();
        };
        insert_activity(
            &conn,
            1,
            "Running",
            day - 2 * DAY_SECONDS,
            1.0,
            "With heart rate",
        );
        segment(1, 125);
        insert_activity(&conn, 3, "Running", day, 1.0, "Today");
        segment(3, 125);
        let fitness = fitness_trend(&conn, 3).unwrap();

        // An earlier run recorded without heart rate doesn't change it.
        insert_activity(
            &conn,
            2,
            "Running",
            day - DAY_SECONDS,
            1.0,
            "Without heart rate",
        );
        segment(2, 0);
        assert_eq!(fitness_trend(&conn, 3).unwrap(), fitness);
    }
}
