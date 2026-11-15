//! Where a plan starts from: the runs of the last six months.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    activity_type::ViewMode,
    analysis::{self, health},
    db::{performances, users, value::opt_f64},
    error::AppResult,
};

const DAY: i64 = 86_400;
const WEEK: i64 = 7 * DAY;
/// Recent weeks the volume, runs per week and longest run are taken from.
pub const RECENT_WEEKS: i64 = 8;
/// The whole look-back.
pub const BASELINE_WEEKS: i64 = 26;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Baseline {
    /// km per week over the last 8 weeks.
    pub weekly_km: f64,
    /// km per week over the last 26 weeks.
    pub weekly_km_long: f64,
    /// Runs per week over the last 8 weeks.
    pub runs_per_week: f64,
    /// km, the longest run of the last 8 weeks, or of the 26 weeks when there was none.
    pub longest_km: f64,
    /// Runs in the last 26 weeks.
    #[ts(type = "number")]
    pub runs: i64,
    /// VO2max from the runs: the higher of the estimate from whole runs and the best effort
    /// over 3 km or more; the profile value without either.
    pub vdot: Option<f64>,
    /// The VDOT is the one entered in the profile.
    pub vdot_from_profile: bool,
    /// Fitness (CTL) and form (TSB) on the last training day.
    pub fitness: Option<f64>,
    pub form: Option<f64>,
}

/// The runs of the 26 weeks up to `now`.
pub fn baseline(conn: &Connection, now: i64) -> AppResult<Baseline> {
    let from = now - BASELINE_WEEKS * WEEK;
    let recent = now - RECENT_WEEKS * WEEK;
    let mut stmt = conn.prepare(
        "SELECT activity_type, date, distance FROM activities
         WHERE date > ?1 AND date <= ?2",
    )?;
    let runs: Vec<(i64, f64)> = stmt
        .query_map([from, now], |row| {
            let activity_type: Option<String> = row.get(0)?;
            Ok((
                ViewMode::from_activity_type(&activity_type.unwrap_or_default()),
                row.get::<_, i64>(1)?,
                opt_f64(row, "distance")?.unwrap_or(0.0),
            ))
        })?
        .filter_map(|row| match row {
            Ok((ViewMode::Running, date, km)) => Some(Ok((date, km))),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<Result<_, _>>()?;

    let recent_runs: Vec<f64> = runs
        .iter()
        .filter(|(date, _)| *date > recent)
        .map(|(_, km)| *km)
        .collect();
    let longest = |kms: &mut dyn Iterator<Item = f64>| kms.fold(0.0, f64::max);
    let longest_recent = longest(&mut recent_runs.iter().copied());
    let longest_km = if longest_recent > 0.0 {
        longest_recent
    } else {
        longest(&mut runs.iter().map(|(_, km)| *km))
    };

    // Whole training runs aren't all-out, so their estimate comes out low for someone who runs
    // easy most of the time; the best stretch of a run is closer to a race effort.
    let estimate = analysis::cardio::estimate(health::run_vo2max(conn, from, now)?)
        .into_iter()
        .chain(best_effort(conn, from, now)?)
        .reduce(f64::max);
    let profile = users::get(conn)?
        .and_then(|u| u.vo2max)
        .filter(|v| *v > 0.0);
    let curve = analysis::fitness_until(conn, now)?;
    let last = curve.last();

    Ok(Baseline {
        weekly_km: recent_runs.iter().sum::<f64>() / RECENT_WEEKS as f64,
        weekly_km_long: runs.iter().map(|(_, km)| km).sum::<f64>() / BASELINE_WEEKS as f64,
        runs_per_week: recent_runs.len() as f64 / RECENT_WEEKS as f64,
        longest_km,
        runs: runs.len() as i64,
        vdot: estimate.or(profile),
        vdot_from_profile: estimate.is_none() && profile.is_some(),
        fitness: last.map(|d| d.fitness),
        form: last.map(|d| d.form),
    })
}

/// VDOT of the fastest best-performance segment of at least 3 km between `from` and `to`.
fn best_effort(conn: &Connection, from: i64, to: i64) -> AppResult<Option<f64>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT b.distance, b.best_time FROM best_performances b
         JOIN activities a ON a.id = b.activity_id
         WHERE b.activity_type = 'Running' AND b.distance >= ?3
            AND a.date > ?1 AND a.date <= ?2 AND {}",
        performances::plausible().replace("activity_type", "b.activity_type")
    ))?;
    let values = stmt
        .query_map(
            rusqlite::params![from, to, analysis::cardio::MIN_RUN_KM],
            |row| {
                let km: f64 = row.get(0)?;
                let pace: f64 = row.get(1)?;
                Ok(analysis::cardio::vdot(km * 1000.0, pace * 60.0 * km))
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(values.into_iter().flatten().reduce(f64::max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    #[test]
    fn counts_recent_runs_only() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let now = 1_800_000_000;
        // Four runs in the last 8 weeks, one 20 weeks ago, a ride and one run too old to count.
        for (id, weeks, km) in [
            (1, 1, 8.0),
            (2, 2, 10.0),
            (3, 3, 6.0),
            (4, 5, 12.0),
            (5, 20, 16.0),
        ] {
            insert_activity(&conn, id, "Running", now - weeks * WEEK, km, "Run");
        }
        insert_activity(&conn, 6, "Biking", now - WEEK, 60.0, "Ride");
        insert_activity(&conn, 7, "Running", now - 30 * WEEK, 30.0, "Old");

        let b = baseline(&conn, now).unwrap();
        assert_eq!(b.weekly_km, 36.0 / 8.0);
        assert_eq!(b.weekly_km_long, 52.0 / 26.0);
        assert_eq!(b.runs_per_week, 0.5);
        assert_eq!(b.longest_km, 12.0);
        assert_eq!(b.runs, 5);
        // The test runs take an hour each, slow enough for a plausible VDOT.
        assert!(b.vdot.is_some());
        assert!(!b.vdot_from_profile);
    }
}
