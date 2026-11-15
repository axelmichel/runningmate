//! Route segments marked on a map (`route_segments`) and every pass over them found in the
//! activities (`segment_efforts`). Ranks are computed on read, so deleting or re-importing an
//! activity re-ranks without anything to update.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, params_from_iter};
use serde::Serialize;
use ts_rs::TS;

use crate::{
    activity_type::ViewMode,
    error::{AppError, AppResult},
    importer::{
        segment_match::{self, Effort, SegmentShape, ShapePoint},
        track::Track,
    },
};

/// Efforts shown on a segment card.
const BEST_SHOWN: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SegmentPoint {
    pub latitude: f64,
    pub longitude: f64,
    /// m.
    pub elevation: f64,
    /// m from the start of the segment.
    pub distance: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RankedEffort {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub activity_id: i64,
    pub activity_title: Option<String>,
    /// Unix timestamp of the start of the pass.
    #[ts(type = "number")]
    pub date: i64,
    /// s.
    pub elapsed: f64,
    pub avg_heart_rate: Option<f64>,
    /// 1 for the fastest; equal times share a rank.
    #[ts(type = "number")]
    pub rank: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RouteSegment {
    #[ts(type = "number")]
    pub id: i64,
    pub title: String,
    pub view: ViewMode,
    #[ts(type = "number | null")]
    pub source_activity_id: Option<i64>,
    /// Unix timestamp.
    #[ts(type = "number")]
    pub created: i64,
    /// m.
    pub distance: f64,
    /// m.
    pub elevation_gain: f64,
    pub elevation_loss: f64,
    /// %.
    pub avg_grade: f64,
    pub max_grade: f64,
    pub min_elevation: f64,
    pub max_elevation: f64,
    pub points: Vec<SegmentPoint>,
    /// Passes over the segment.
    #[ts(type = "number")]
    pub tries: i64,
    /// Activities with at least one pass.
    #[ts(type = "number")]
    pub activities: i64,
    /// The fastest passes, at most three.
    pub best: Vec<RankedEffort>,
    /// The most recent pass.
    pub latest: Option<RankedEffort>,
    /// The fastest pass since 1 January (local time).
    pub best_this_year: Option<RankedEffort>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SegmentDetail {
    pub segment: RouteSegment,
    /// Every pass, fastest first.
    pub leaderboard: Vec<RankedEffort>,
}

/// A pass of one activity over a segment, with its place among all passes.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivitySegmentEffort {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub segment_id: i64,
    pub title: String,
    /// m.
    pub distance: f64,
    /// %.
    pub avg_grade: f64,
    /// m.
    pub elevation_gain: f64,
    /// Position within the activity, km.
    pub start_km: f64,
    pub end_km: f64,
    /// s.
    pub elapsed: f64,
    pub avg_heart_rate: Option<f64>,
    #[ts(type = "number")]
    pub rank: i64,
    #[ts(type = "number")]
    pub tries: i64,
}

fn view_name(view: ViewMode) -> String {
    format!("{view:?}")
}

fn parse_view(name: &str) -> ViewMode {
    match name {
        "Running" => ViewMode::Running,
        "Cycling" => ViewMode::Cycling,
        "Walking" => ViewMode::Walking,
        _ => ViewMode::All,
    }
}

/// Efforts with their rank and the number of passes per segment, for efforts of activities that
/// still exist.
const RANKED: &str = "WITH ranked AS (
    SELECT e.id, e.segment_id, e.activity_id, a.title AS activity_title, e.date, e.elapsed,
        e.avg_heart_rate, e.start_km, e.end_km,
        RANK() OVER (PARTITION BY e.segment_id ORDER BY e.elapsed) AS rank,
        COUNT(*) OVER (PARTITION BY e.segment_id) AS tries
    FROM segment_efforts e JOIN activities a ON a.id = e.activity_id
)";

fn ranked_from_row(row: &Row) -> rusqlite::Result<RankedEffort> {
    Ok(RankedEffort {
        id: row.get("id")?,
        activity_id: row.get("activity_id")?,
        activity_title: row.get("activity_title")?,
        date: row.get("date")?,
        elapsed: row.get("elapsed")?,
        avg_heart_rate: row.get("avg_heart_rate")?,
        rank: row.get("rank")?,
    })
}

fn points_from_json(json: &str) -> AppResult<Vec<ShapePoint>> {
    Ok(serde_json::from_str(json)?)
}

fn segment_from_row(row: &Row) -> rusqlite::Result<(RouteSegment, String)> {
    Ok((
        RouteSegment {
            id: row.get("id")?,
            title: row.get("title")?,
            view: parse_view(&row.get::<_, String>("view")?),
            source_activity_id: row.get("source_activity_id")?,
            created: row.get("created")?,
            distance: row.get("distance")?,
            elevation_gain: row.get("elevation_gain")?,
            elevation_loss: row.get("elevation_loss")?,
            avg_grade: row.get("avg_grade")?,
            max_grade: row.get("max_grade")?,
            min_elevation: row.get("min_elevation")?,
            max_elevation: row.get("max_elevation")?,
            points: Vec::new(),
            tries: row.get("tries")?,
            activities: row.get("activity_count")?,
            best: Vec::new(),
            latest: None,
            best_this_year: None,
        },
        row.get("points")?,
    ))
}

const SEGMENT_COLUMNS: &str = "s.id, s.title, s.view, s.source_activity_id, s.created, s.distance,
    s.elevation_gain, s.elevation_loss, s.avg_grade, s.max_grade, s.min_elevation,
    s.max_elevation, s.points,
    (SELECT COUNT(*) FROM segment_efforts e JOIN activities a ON a.id = e.activity_id
     WHERE e.segment_id = s.id) AS tries,
    (SELECT COUNT(DISTINCT e.activity_id) FROM segment_efforts e
     JOIN activities a ON a.id = e.activity_id WHERE e.segment_id = s.id) AS activity_count";

/// Unix timestamp of 1 January, 00:00 local time, of the current year.
fn start_of_year() -> i64 {
    use chrono::{Datelike, Local, TimeZone};
    let year = Local::now().year();
    Local
        .with_ymd_and_hms(year, 1, 1, 0, 0, 0)
        .earliest()
        .map_or(0, |start| start.timestamp())
}

/// Fills in the three fastest, the latest and this year's fastest from every pass, given
/// fastest first.
fn set_highlights(segment: &mut RouteSegment, efforts: &[RankedEffort], year_start: i64) {
    segment.best = efforts.iter().take(BEST_SHOWN).cloned().collect();
    segment.latest = efforts.iter().max_by_key(|e| e.date).cloned();
    segment.best_this_year = efforts.iter().find(|e| e.date >= year_start).cloned();
}

fn with_points(segment: (RouteSegment, String)) -> AppResult<RouteSegment> {
    let (mut segment, json) = segment;
    segment.points = points_from_json(&json)?
        .into_iter()
        .map(|p| SegmentPoint {
            latitude: p.latitude(),
            longitude: p.longitude(),
            elevation: p.elevation(),
            distance: p.distance(),
        })
        .collect();
    Ok(segment)
}

/// Stores a new segment and returns its id. Its passes are found separately.
pub fn create(
    conn: &Connection,
    title: &str,
    view: ViewMode,
    source_activity_id: i64,
    shape: &SegmentShape,
    created: i64,
) -> AppResult<i64> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Invalid("a segment needs a title".into()));
    }
    conn.execute(
        "INSERT INTO route_segments (title, view, source_activity_id, created, distance,
            elevation_gain, elevation_loss, avg_grade, max_grade, min_elevation, max_elevation,
            points)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            title,
            view_name(view),
            source_activity_id,
            created,
            shape.distance,
            shape.elevation_gain,
            shape.elevation_loss,
            shape.avg_grade,
            shape.max_grade,
            shape.min_elevation,
            shape.max_elevation,
            serde_json::to_string(&shape.points)?,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, title: &str) -> AppResult<()> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::Invalid("a segment needs a title".into()));
    }
    let changed = conn.execute(
        "UPDATE route_segments SET title = ?1 WHERE id = ?2",
        params![title, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("segment {id}")));
    }
    Ok(())
}

pub fn delete(conn: &mut Connection, id: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM segment_efforts WHERE segment_id = ?1", [id])?;
    tx.execute("DELETE FROM route_segments WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(())
}

/// The shape of a segment, for matching.
fn shape_from_row(row: &Row) -> rusqlite::Result<(i64, String, SegmentShape)> {
    Ok((
        row.get("id")?,
        row.get("points")?,
        SegmentShape {
            points: Vec::new(),
            distance: row.get("distance")?,
            elevation_gain: row.get("elevation_gain")?,
            elevation_loss: row.get("elevation_loss")?,
            avg_grade: row.get("avg_grade")?,
            max_grade: row.get("max_grade")?,
            min_elevation: row.get("min_elevation")?,
            max_elevation: row.get("max_elevation")?,
        },
    ))
}

fn shapes(
    conn: &Connection,
    sql: &str,
    param: &dyn rusqlite::ToSql,
) -> AppResult<Vec<(i64, SegmentShape)>> {
    let rows: Vec<(i64, String, SegmentShape)> = conn
        .prepare(sql)?
        .query_map([param], shape_from_row)?
        .collect::<Result<_, _>>()?;
    rows.into_iter()
        .map(|(id, json, mut shape)| {
            shape.points = points_from_json(&json)?;
            Ok((id, shape))
        })
        .collect()
}

const SHAPE_COLUMNS: &str = "id, points, distance, elevation_gain, elevation_loss, avg_grade,
    max_grade, min_elevation, max_elevation";

/// View and shape of one segment.
pub fn shape(conn: &Connection, id: i64) -> AppResult<(ViewMode, SegmentShape)> {
    let view: String = conn
        .query_row(
            "SELECT view FROM route_segments WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("segment {id}")))?;
    let (_, shape) = shapes(
        conn,
        &format!("SELECT {SHAPE_COLUMNS} FROM route_segments WHERE id = ?1"),
        &id,
    )?
    .remove(0);
    Ok((parse_view(&view), shape))
}

fn insert_efforts(
    tx: &Transaction,
    segment_id: i64,
    activity_id: i64,
    efforts: &[Effort],
) -> AppResult<()> {
    let mut insert = tx.prepare(
        "INSERT INTO segment_efforts (segment_id, activity_id, start_km, end_km, elapsed,
            avg_heart_rate, date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for e in efforts {
        insert.execute(params![
            segment_id,
            activity_id,
            e.start_km,
            e.end_km,
            e.elapsed,
            e.avg_heart_rate,
            e.date
        ])?;
    }
    Ok(())
}

/// Finds the activity's passes over every segment of its view, replacing the stored ones.
pub fn store_efforts(tx: &Transaction, activity_id: i64, track: &Track) -> AppResult<()> {
    tx.execute(
        "DELETE FROM segment_efforts WHERE activity_id = ?1",
        [activity_id],
    )?;
    let segments = shapes(
        tx,
        &format!("SELECT {SHAPE_COLUMNS} FROM route_segments WHERE view = ?1"),
        &view_name(track.view),
    )?;
    for (segment_id, shape) in segments {
        let efforts = segment_match::find_efforts(&shape, track);
        insert_efforts(tx, segment_id, activity_id, &efforts)?;
    }
    Ok(())
}

/// Replaces the passes of one activity over one segment.
pub fn store_segment_efforts(
    tx: &Transaction,
    segment_id: i64,
    activity_id: i64,
    efforts: &[Effort],
) -> AppResult<()> {
    tx.execute(
        "DELETE FROM segment_efforts WHERE segment_id = ?1 AND activity_id = ?2",
        [segment_id, activity_id],
    )?;
    insert_efforts(tx, segment_id, activity_id, efforts)
}

/// Activities of the segment's view with an archived file that come near it: one of their split
/// starts lies within a split's length of the segment: `(activity id, file id)`, newest first.
pub fn candidate_activities(conn: &Connection, segment_id: i64) -> AppResult<Vec<(i64, String)>> {
    let (view, shape) = shape(conn, segment_id)?;
    let split_km = if view == ViewMode::Cycling { 5.0 } else { 1.0 };
    let (south, west, north, east) = segment_match::bounding_box(&shape.points, split_km * 1.5);
    let types = view.allowed_types();
    if types.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; types.len()].join(", ");
    let sql = format!(
        "SELECT a.id, a.file_id FROM activities a
         WHERE a.file_id IS NOT NULL AND a.activity_type IN ({placeholders})
           AND EXISTS (SELECT 1 FROM activity_details d
               WHERE d.activity_id = CAST(a.id AS TEXT)
                 AND d.seg_latitude BETWEEN ? AND ? AND d.seg_longitude BETWEEN ? AND ?)
         ORDER BY a.date DESC"
    );
    let mut values: Vec<rusqlite::types::Value> =
        types.iter().map(|t| (*t).to_string().into()).collect();
    values.extend([south, north, west, east].map(Into::into));
    Ok(conn
        .prepare(&sql)?
        .query_map(params_from_iter(values), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<Result<_, _>>()?)
}

/// Segments of a view (`All` for every view) whose title contains `search`, by title, each with
/// its three fastest passes, its latest and this year's fastest.
pub fn list(conn: &Connection, view: ViewMode, search: &str) -> AppResult<Vec<RouteSegment>> {
    let pattern = format!("%{}%", search.trim());
    let rows: Vec<(RouteSegment, String)> = conn
        .prepare(&format!(
            "SELECT {SEGMENT_COLUMNS} FROM route_segments s
             WHERE (?1 = 'All' OR s.view = ?1) AND s.title LIKE ?2
             ORDER BY s.title COLLATE NOCASE"
        ))?
        .query_map(params![view_name(view), pattern], segment_from_row)?
        .collect::<Result<_, _>>()?;
    let mut efforts: HashMap<i64, Vec<RankedEffort>> = HashMap::new();
    let mut stmt = conn.prepare(&format!(
        "{RANKED} SELECT * FROM ranked ORDER BY segment_id, rank, date"
    ))?;
    let mut rows_of_efforts = stmt.query([])?;
    while let Some(row) = rows_of_efforts.next()? {
        let segment_id: i64 = row.get("segment_id")?;
        efforts
            .entry(segment_id)
            .or_default()
            .push(ranked_from_row(row)?);
    }
    let year_start = start_of_year();
    rows.into_iter()
        .map(|row| {
            let mut segment = with_points(row)?;
            let passes = efforts.remove(&segment.id).unwrap_or_default();
            set_highlights(&mut segment, &passes, year_start);
            Ok(segment)
        })
        .collect()
}

/// One segment with every pass, fastest first.
pub fn get(conn: &Connection, id: i64) -> AppResult<SegmentDetail> {
    let row = conn
        .query_row(
            &format!("SELECT {SEGMENT_COLUMNS} FROM route_segments s WHERE s.id = ?1"),
            [id],
            segment_from_row,
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound(format!("segment {id}")))?;
    let mut segment = with_points(row)?;
    let leaderboard: Vec<RankedEffort> = conn
        .prepare(&format!(
            "{RANKED} SELECT * FROM ranked WHERE segment_id = ?1 ORDER BY rank, date"
        ))?
        .query_map([id], ranked_from_row)?
        .collect::<Result<_, _>>()?;
    set_highlights(&mut segment, &leaderboard, start_of_year());
    Ok(SegmentDetail {
        segment,
        leaderboard,
    })
}

/// The activity's passes over segments, in the order they were ridden or run.
pub fn for_activity(conn: &Connection, activity_id: i64) -> AppResult<Vec<ActivitySegmentEffort>> {
    Ok(conn
        .prepare(&format!(
            "{RANKED} SELECT r.id, r.segment_id, s.title, s.distance, s.avg_grade,
                s.elevation_gain, r.start_km, r.end_km, r.elapsed, r.avg_heart_rate, r.rank,
                r.tries
             FROM ranked r JOIN route_segments s ON s.id = r.segment_id
             WHERE r.activity_id = ?1
             ORDER BY r.start_km"
        ))?
        .query_map([activity_id], |row| {
            Ok(ActivitySegmentEffort {
                id: row.get("id")?,
                segment_id: row.get("segment_id")?,
                title: row.get("title")?,
                distance: row.get("distance")?,
                avg_grade: row.get("avg_grade")?,
                elevation_gain: row.get("elevation_gain")?,
                start_km: row.get("start_km")?,
                end_km: row.get("end_km")?,
                elapsed: row.get("elapsed")?,
                avg_heart_rate: row.get("avg_heart_rate")?,
                rank: row.get("rank")?,
                tries: row.get("tries")?,
            })
        })?
        .collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{Db, activities},
        importer::{self, Processed, segment_match},
    };

    const FIXTURE: &str = include_str!("../../tests/fixtures/synthetic_run.tcx");

    /// The fixture run, `k` × 10 % slower than recorded.
    fn slowed(k: usize) -> Processed {
        let mut processed = importer::process(&format!("run{k}"), FIXTURE, 70.0).unwrap();
        let mut points = processed.track.points.clone();
        let start = points[0].time;
        for p in &mut points {
            let offset = (p.time - start).as_seconds_f64() * (1.0 + k as f64 * 0.1);
            p.time = start + chrono::TimeDelta::milliseconds((offset * 1000.0) as i64);
        }
        processed.track = Track::new(points, processed.view, 70.0);
        processed
    }

    /// `count` stored runs, each slower than the one before, and a segment over the middle of
    /// the first, created after them.
    fn seeded(count: usize) -> (Db, Vec<(i64, Processed)>, i64) {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        let runs: Vec<(i64, Processed)> = (0..count)
            .map(|k| {
                let processed = slowed(k);
                (
                    importer::store(&mut conn, &processed, None, "en").unwrap(),
                    processed,
                )
            })
            .collect();
        let source = &runs[0].1.track;
        let total = source.distance_km[source.len() - 1];
        let shape = segment_match::cut(source, total * 0.2, total * 0.6).unwrap();
        let segment = create(&conn, "Hill", source.view, runs[0].0, &shape, 0).unwrap();
        drop(conn);
        (db, runs, segment)
    }

    fn match_all(db: &Db, runs: &[(i64, Processed)]) {
        let mut conn = db.conn().unwrap();
        for (id, processed) in runs {
            let tx = conn.transaction().unwrap();
            store_efforts(&tx, *id, &processed.track).unwrap();
            tx.commit().unwrap();
        }
    }

    #[test]
    fn ranks_passes_by_time() {
        let (db, runs, segment) = seeded(3);
        let ids: Vec<i64> = runs.iter().map(|(id, _)| *id).collect();
        let mut conn = db.conn().unwrap();
        // Stored before the segment existed, so nothing is matched yet, but all are candidates.
        assert!(get(&conn, segment).unwrap().leaderboard.is_empty());
        assert_eq!(candidate_activities(&conn, segment).unwrap().len(), 3);
        drop(conn);
        match_all(&db, &runs);
        conn = db.conn().unwrap();

        let detail = get(&conn, segment).unwrap();
        assert_eq!((detail.segment.tries, detail.segment.activities), (3, 3));
        let order: Vec<(i64, i64)> = detail
            .leaderboard
            .iter()
            .map(|e| (e.activity_id, e.rank))
            .collect();
        assert_eq!(order, [(ids[0], 1), (ids[1], 2), (ids[2], 3)]);
        let efforts = for_activity(&conn, ids[1]).unwrap();
        assert_eq!(efforts.len(), 1);
        assert_eq!((efforts[0].rank, efforts[0].tries), (2, 3));

        let listed = list(&conn, ViewMode::All, "hi").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].best.len(), 3);
        assert!(!listed[0].points.is_empty());
        assert!(list(&conn, ViewMode::Cycling, "").unwrap().is_empty());
        assert!(list(&conn, ViewMode::All, "valley").unwrap().is_empty());

        // Deleting the fastest activity moves everyone up.
        activities::delete(&mut conn, ids[0]).unwrap();
        let ranks: Vec<(i64, i64)> = get(&conn, segment)
            .unwrap()
            .leaderboard
            .iter()
            .map(|e| (e.activity_id, e.rank))
            .collect();
        assert_eq!(ranks, [(ids[1], 1), (ids[2], 2)]);
    }

    #[test]
    fn latest_and_best_this_year() {
        let effort = |id: i64, date: i64, rank: i64| RankedEffort {
            id,
            activity_id: id,
            activity_title: None,
            date,
            elapsed: rank as f64 * 10.0,
            avg_heart_rate: None,
            rank,
        };
        let (db, runs, id) = seeded(1);
        match_all(&db, &runs);
        let mut segment = get(&db.conn().unwrap(), id).unwrap().segment;
        // Fastest first: two old record passes, then this year's, the latest last year.
        let efforts = [
            effort(1, 100, 1),
            effort(2, 200, 2),
            effort(3, 1_000, 3),
            effort(4, 2_000, 4),
            effort(5, 900, 5),
        ];
        set_highlights(&mut segment, &efforts, 950);
        assert_eq!(
            segment.best.iter().map(|e| e.id).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(segment.latest.as_ref().map(|e| e.id), Some(4));
        assert_eq!(segment.best_this_year.as_ref().map(|e| e.id), Some(3));
        set_highlights(&mut segment, &efforts[..2], 950);
        assert!(segment.best_this_year.is_none());
        set_highlights(&mut segment, &[], 950);
        assert!(segment.latest.is_none() && segment.best.is_empty());
    }

    #[test]
    fn stored_activities_are_matched_once() {
        let (db, runs, segment) = seeded(1);
        let mut conn = db.conn().unwrap();
        // A new import and a re-import go through `store`, which matches without doubling.
        let new = importer::store(&mut conn, &slowed(1), None, "en").unwrap();
        importer::store(&mut conn, &runs[0].1, Some(runs[0].0), "en").unwrap();
        importer::store(&mut conn, &runs[0].1, Some(runs[0].0), "en").unwrap();
        let detail = get(&conn, segment).unwrap();
        assert_eq!(detail.segment.tries, 2);
        assert_eq!(detail.leaderboard[1].activity_id, new);
    }

    #[test]
    fn rename_and_delete() {
        let (db, runs, segment) = seeded(1);
        match_all(&db, &runs);
        let mut conn = db.conn().unwrap();
        assert!(rename(&conn, segment, "  ").is_err());
        rename(&conn, segment, "Steep bit").unwrap();
        assert_eq!(get(&conn, segment).unwrap().segment.title, "Steep bit");
        delete(&mut conn, segment).unwrap();
        assert!(matches!(get(&conn, segment), Err(AppError::NotFound(_))));
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM segment_efforts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }
}
