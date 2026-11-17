//! The "Health data" page: fitness and heart rate over a chosen period, across all activity
//! types (the detail window shows the same for the 90 days up to one activity).

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use chrono::DateTime;

use super::{
    cardio::{self, CardioRating},
    fitness::FitnessDay,
    fitness_until, in_period, segment_time, zones,
    zones::ZoneTime,
};
use crate::{
    activity_type::ViewMode,
    db::{users, value::opt_f64},
    error::AppResult,
};

/// Activities in the rolling heart-rate mean, as in the detail window's trend.
const TREND_WINDOW: usize = 30;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HealthHeartPoint {
    #[ts(type = "number")]
    pub activity_id: i64,
    /// Unix timestamp of the activity.
    #[ts(type = "number")]
    pub date: i64,
    pub view_mode: ViewMode,
    pub avg_heart_rate: f64,
    /// Rolling mean over the last 30 activities of any type.
    pub trend: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HealthReport {
    pub fitness: Vec<FitnessDay>,
    /// The highest fitness ever reached up to the end of the period, so the period's values
    /// can be read as a share of the personal best (the load numbers mean nothing on their own).
    pub fitness_peak: Option<FitnessDay>,
    pub heart_rate: Vec<HealthHeartPoint>,
    /// Zone 1, 3 and 5 bounds drawn as reference lines (min, ideal, max).
    #[ts(type = "[number, number, number] | null")]
    pub references: Option<[i64; 3]>,
    /// Time in heart-rate zones from the per-km segments; `None` while the user's zone method has no zones.
    pub zones: Option<Vec<ZoneTime>>,
    /// Activities in the period (with or without heart rate).
    #[ts(type = "number")]
    pub activities: i64,
    pub cardio: CardioFitness,
}

/// Where a VO2max value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Vo2maxSource {
    /// Estimated from the period's runs.
    Runs,
    /// Entered in the user settings (no runs to estimate from).
    Profile,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardioFitness {
    /// ml/kg/min; `None` without enough runs and without a value in the profile.
    pub vo2max: Option<f64>,
    pub source: Option<Vo2maxSource>,
    /// Runs of at least 3 km in the period that the estimate is based on.
    #[ts(type = "number")]
    pub runs: i64,
    /// Age at the end of the period.
    #[ts(type = "number | null")]
    pub age: Option<i64>,
    /// Comparison with the same age group and sex; needs a VO2max, age and sex.
    pub rating: Option<CardioRating>,
}

fn heart_rate_points(conn: &Connection, from: i64, to: i64) -> AppResult<Vec<HealthHeartPoint>> {
    let union = [ViewMode::Running, ViewMode::Walking, ViewMode::Cycling]
        .iter()
        .filter_map(|view| {
            let table = view.table()?;
            Some(format!(
                "SELECT activities.id AS id, activities.date AS date, '{view:?}' AS view,
                    {table}.avg_heart_rate AS hr
                 FROM {table} JOIN activities ON activities.id = {table}.activity_id
                 WHERE activities.date BETWEEN ?1 AND ?2 AND {table}.avg_heart_rate > 0"
            ))
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    let mut stmt = conn.prepare(&format!("{union} ORDER BY date"))?;
    let raw: Vec<(i64, i64, ViewMode, f64)> = stmt
        .query_map([from, to], |row| {
            let view: String = row.get("view")?;
            Ok((
                row.get("id")?,
                row.get("date")?,
                match view.as_str() {
                    "Running" => ViewMode::Running,
                    "Walking" => ViewMode::Walking,
                    _ => ViewMode::Cycling,
                },
                opt_f64(row, "hr")?.unwrap_or(0.0),
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(raw
        .iter()
        .enumerate()
        .map(|(i, (activity_id, date, view_mode, hr))| {
            let slice = &raw[(i + 1).saturating_sub(TREND_WINDOW)..=i];
            HealthHeartPoint {
                activity_id: *activity_id,
                date: *date,
                view_mode: *view_mode,
                avg_heart_rate: *hr,
                trend: slice.iter().map(|(.., h)| h).sum::<f64>() / slice.len() as f64,
            }
        })
        .collect())
}

/// `(average heart rate, seconds)` of every segment in the period.
fn segment_heart_rates(conn: &Connection, from: i64, to: i64) -> AppResult<Vec<(f64, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT seg_avg_heart_rate, seg_time_start, seg_time_end
         FROM activity_details JOIN activities ON activity_details.activity_id = activities.id
         WHERE activities.date BETWEEN ?1 AND ?2 AND seg_avg_heart_rate > 0",
    )?;
    let rows = stmt.query_map([from, to], |row| {
        let hr = opt_f64(row, "seg_avg_heart_rate")?;
        let start: Option<String> = row.get("seg_time_start")?;
        let end: Option<String> = row.get("seg_time_end")?;
        Ok(
            match (
                hr,
                start.as_deref().and_then(segment_time),
                end.as_deref().and_then(segment_time),
            ) {
                (Some(hr), Some(s), Some(e)) => Some((hr, (e - s) as f64)),
                _ => None,
            },
        )
    })?;
    Ok(rows
        .filter_map(|r| r.transpose())
        .collect::<Result<_, _>>()?)
}

/// Per-run VO2max values of the period's runs of at least [`cardio::MIN_RUN_KM`].
pub(crate) fn run_vo2max(conn: &Connection, from: i64, to: i64) -> AppResult<Vec<f64>> {
    let mut stmt = conn.prepare(
        "SELECT activities.distance AS km, activities.duration AS seconds, runs.pause AS pause
         FROM runs JOIN activities ON activities.id = runs.activity_id
         WHERE activities.date BETWEEN ?1 AND ?2 AND activities.distance >= ?3",
    )?;
    let rows = stmt.query_map(rusqlite::params![from, to, cardio::MIN_RUN_KM], |row| {
        let km = opt_f64(row, "km")?.unwrap_or(0.0);
        let seconds = opt_f64(row, "seconds")?.unwrap_or(0.0);
        let pause: Option<String> = row.get("pause")?;
        let moving = seconds - cardio::pause_seconds(pause.as_deref().unwrap_or_default());
        Ok(cardio::vdot(km * 1000.0, moving))
    })?;
    Ok(rows
        .filter_map(|r| r.transpose())
        .collect::<Result<_, _>>()?)
}

fn cardio_fitness(
    conn: &Connection,
    user: Option<&users::User>,
    from: i64,
    to: i64,
) -> AppResult<CardioFitness> {
    let values = run_vo2max(conn, from, to)?;
    let runs = values.len() as i64;
    let (vo2max, source) = match cardio::estimate(values) {
        Some(value) => (Some(value), Some(Vo2maxSource::Runs)),
        None => match user.and_then(|u| u.vo2max).filter(|v| *v > 0.0) {
            Some(value) => (Some(value), Some(Vo2maxSource::Profile)),
            None => (None, None),
        },
    };
    let end = DateTime::from_timestamp(to, 0).map(|t| t.date_naive());
    let age = user.and_then(|u| {
        u.birthday
            .as_deref()
            .zip(end)
            .and_then(|(birthday, end)| users::age_on(birthday, end))
            .or(u.age)
    });
    let rating = match (vo2max, age, user.and_then(|u| u.gender)) {
        (Some(vo2max), Some(age), Some(gender)) => Some(cardio::rating(vo2max, age, gender)),
        _ => None,
    };
    Ok(CardioFitness {
        vo2max,
        source,
        runs,
        age,
        rating,
    })
}

pub fn report(conn: &Connection, from: i64, to: i64) -> AppResult<HealthReport> {
    let user = users::get(conn)?;
    let curve = fitness_until(conn, to)?;
    // `users::get` resolves the zones for the user's method, as `users::zones` does.
    let bounds = user.as_ref().and_then(|u| u.zones);
    let zones = match bounds {
        Some(bounds) => Some(zones::from_segments(
            &segment_heart_rates(conn, from, to)?,
            &bounds,
        )),
        None => None,
    };
    let activities = conn.query_row(
        "SELECT COUNT(*) FROM activities WHERE date BETWEEN ?1 AND ?2",
        [from, to],
        |row| row.get(0),
    )?;
    Ok(HealthReport {
        fitness_peak: curve
            .iter()
            .max_by(|a, b| a.fitness.total_cmp(&b.fitness))
            .cloned(),
        fitness: in_period(curve, from),
        heart_rate: heart_rate_points(conn, from, to)?,
        references: bounds.map(|z| [z.upper[0], z.upper[2], z.upper[4]]),
        zones,
        activities,
        cardio: cardio_fitness(conn, user.as_ref(), from, to)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    #[test]
    fn collects_heart_rate_across_types_in_the_period() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 1, "Running", 1_000, 5.0, "Run");
        insert_activity(&conn, 2, "Cycling", 2_000, 20.0, "Ride");
        insert_activity(&conn, 3, "Running", 9_000, 5.0, "Later run");
        conn.execute_batch(
            "UPDATE runs SET avg_heart_rate = 150 WHERE activity_id = 1;
             UPDATE activities SET duration = 1500 WHERE id = 1;
             UPDATE cycling SET avg_heart_rate = 130 WHERE activity_id = 2;",
        )
        .unwrap();

        let report = report(&conn, 0, 5_000).unwrap();
        assert_eq!(report.activities, 2);
        let points: Vec<_> = report
            .heart_rate
            .iter()
            .map(|p| (p.activity_id, p.view_mode, p.avg_heart_rate, p.trend))
            .collect();
        assert_eq!(
            points,
            [
                (1, ViewMode::Running, 150.0, 150.0),
                (2, ViewMode::Cycling, 130.0, 140.0),
            ]
        );
        assert_eq!(report.zones, None);
        // No segments, so no training load.
        assert_eq!(report.fitness_peak, None);
        // One run (5 km in 25 minutes) is too few for an estimate, and there is no profile.
        assert_eq!(report.cardio.runs, 1);
        assert_eq!(report.cardio.vo2max, None);
    }
}
