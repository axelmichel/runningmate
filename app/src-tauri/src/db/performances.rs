//! Best segment paces (`best_performances` table), ported from the read side of
//! `processing/best_performances.py`. `best_time` holds the average pace in min/km.

use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use super::value::{opt_f64, opt_i64, opt_string};
use crate::{activity_type::ViewMode, error::AppResult, importer::track::pace_limits};

/// Segment distances (km) checked per view, from `BestSegmentFinder.SEGMENT_DISTANCES`.
pub fn segment_distances(view: ViewMode) -> &'static [f64] {
    match view {
        ViewMode::Running => &[1.0, 5.0, 10.0, 21.0, 42.0],
        ViewMode::Walking => &[1.0, 5.0, 10.0, 15.0, 20.0, 30.0, 50.0],
        ViewMode::Cycling => &[5.0, 25.0, 50.0, 75.0, 100.0, 150.0, 200.0],
        ViewMode::All => &[],
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BestPerformance {
    #[ts(type = "number")]
    pub activity_id: i64,
    /// Segment length in km.
    pub distance: f64,
    /// Average pace over the segment in min/km.
    pub pace: f64,
    /// Start of the segment, as stored ("YYYY-MM-DD HH:MM:SS+00:00").
    pub start_time: Option<String>,
}

/// SQL condition keeping paces a person can manage: not faster than the per-point pace limit of
/// the activity type (`importer::track::pace_limits`). A file whose clock jumps back produces
/// segments with negative or tiny times, and a ride recorded as a run produces world records.
pub(crate) fn plausible() -> String {
    let limits = [ViewMode::Running, ViewMode::Cycling, ViewMode::Walking]
        .map(|view| format!("WHEN '{view:?}' THEN {:?}", pace_limits(view).0))
        .join(" ");
    format!("best_time >= CASE activity_type {limits} ELSE 0 END AND best_time > 0")
}

fn read(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> AppResult<Vec<BestPerformance>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, |row| {
        Ok(BestPerformance {
            activity_id: opt_i64(row, "activity_id")?.unwrap_or_default(),
            distance: opt_f64(row, "distance")?.unwrap_or_default(),
            pace: opt_f64(row, "best_time")?.unwrap_or_default(),
            start_time: opt_string(row, "date_time")?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Best segments of one activity, shortest distance first.
pub fn for_activity(conn: &Connection, activity_id: i64) -> AppResult<Vec<BestPerformance>> {
    read(
        conn,
        &format!(
            "SELECT activity_id, distance, MIN(best_time) AS best_time, date_time
             FROM best_performances WHERE activity_id = ?1 AND {}
             GROUP BY distance ORDER BY distance",
            plausible()
        ),
        [activity_id],
    )
}

/// Top three paces per segment distance across all activities of a view.
pub fn top(conn: &Connection, view: ViewMode) -> AppResult<Vec<BestPerformance>> {
    let mut result = Vec::new();
    let view_name = serde_json::to_value(view)?
        .as_str()
        .unwrap_or_default()
        .to_string();
    for distance in segment_distances(view) {
        result.extend(read(
            conn,
            &format!(
                "SELECT activity_id, distance, best_time, date_time FROM best_performances
                 WHERE activity_type = ?1 AND distance = ?2 AND {}
                 ORDER BY best_time ASC LIMIT 3",
                plausible()
            ),
            rusqlite::params![view_name, distance],
        )?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn reads_per_activity_and_top_three() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        for (activity, distance, pace) in [
            (1, 1.0, 5.5),
            (1, 5.0, 6.0),
            (2, 1.0, 5.0),
            (3, 1.0, 5.2),
            (4, 1.0, 5.9),
        ] {
            conn.execute(
                "INSERT INTO best_performances (activity_id, distance, best_time, date_time,
                    activity_type) VALUES (?1, ?2, ?3, '2025-01-01 10:00:00+00:00', 'Running')",
                rusqlite::params![activity, distance, pace],
            )
            .unwrap();
        }
        let own = for_activity(&conn, 1).unwrap();
        assert_eq!(
            own.iter().map(|p| p.distance).collect::<Vec<_>>(),
            [1.0, 5.0]
        );

        let top = top(&conn, ViewMode::Running).unwrap();
        let one_k: Vec<_> = top
            .iter()
            .filter(|p| p.distance == 1.0)
            .map(|p| p.activity_id)
            .collect();
        assert_eq!(one_k, [2, 3, 1]);
        assert_eq!(top.iter().filter(|p| p.distance == 5.0).count(), 1);
    }

    #[test]
    fn leaves_out_impossible_records() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        conn.execute_batch(
            "INSERT INTO best_performances (activity_id, distance, best_time, activity_type,
                date_time)
             VALUES (1, 5.0, 5.5, 'Running', ''), (2, 5.0, 0.15, 'Running', ''),
                    (3, 5.0, -0.2, 'Running', ''), (4, 5.0, 2.0, 'Cycling', ''),
                    (2, 1.0, 0.1, 'Running', '');",
        )
        .unwrap();
        let running: Vec<_> = top(&conn, ViewMode::Running)
            .unwrap()
            .iter()
            .map(|p| p.activity_id)
            .collect();
        assert_eq!(running, [1]);
        assert_eq!(top(&conn, ViewMode::Cycling).unwrap().len(), 1);
        assert!(for_activity(&conn, 2).unwrap().is_empty());
    }
}
