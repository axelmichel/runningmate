//! Simplified GPS tracks for the route heat map. Each activity's track is simplified once,
//! stored as an encoded polyline and only rebuilt when it is missing or [`TRACK_VERSION`]
//! changes, so showing the heat map again only reads stored rows.

use std::{collections::HashMap, path::Path, thread};

use rusqlite::{Connection, params, params_from_iter, types::Value};
use serde::Serialize;
use ts_rs::TS;

use super::{Db, activities::ActivityFilter};
use crate::{
    activity_type::ViewMode,
    error::AppResult,
    importer::{
        archive, outline,
        tcx::{self, RawPoint},
    },
};

/// Raise to rebuild every stored track on the next heat map view.
const TRACK_VERSION: i64 = 1;
/// Points closer than this (metres) to the simplified line are dropped.
const EPSILON_METRES: f64 = 5.0;
const METRES_PER_DEGREE: f64 = 111_320.0;

/// Encoded polylines (Google format, 5 digits) of the activities in a heat map selection.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RouteHeatmap {
    pub tracks: Vec<String>,
}

/// Simplified, encoded track; empty with fewer than two points.
pub fn encode_track(points: &[RawPoint]) -> String {
    if points.len() < 2 {
        return String::new();
    }
    // Equirectangular metres around the first point: exact to invert, and accurate enough over
    // the extent of one activity to pick the points to keep.
    let scale = points[0].latitude.to_radians().cos() * METRES_PER_DEGREE;
    let xy: Vec<(f64, f64)> = points
        .iter()
        .map(|p| (p.longitude * scale, p.latitude * METRES_PER_DEGREE))
        .collect();
    let kept: Vec<(f64, f64)> = outline::simplify(&xy, EPSILON_METRES)
        .into_iter()
        .map(|(x, y)| (y / METRES_PER_DEGREE, x / scale))
        .collect();
    encode_polyline(&kept)
}

/// Google polyline encoding of `(latitude, longitude)` pairs.
fn encode_polyline(points: &[(f64, f64)]) -> String {
    fn push(value: i64, out: &mut String) {
        let mut v = if value < 0 { !(value << 1) } else { value << 1 };
        while v >= 0x20 {
            out.push(char::from((0x20 | (v & 0x1f)) as u8 + 63));
            v >>= 5;
        }
        out.push(char::from(v as u8 + 63));
    }
    let mut out = String::new();
    let (mut last_lat, mut last_lon) = (0, 0);
    for (lat, lon) in points {
        let (lat, lon) = ((lat * 1e5).round() as i64, (lon * 1e5).round() as i64);
        push(lat - last_lat, &mut out);
        push(lon - last_lon, &mut out);
        (last_lat, last_lon) = (lat, lon);
    }
    out
}

/// Stores (or replaces) an activity's track.
pub fn store(conn: &Connection, activity_id: i64, polyline: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO activity_tracks (activity_id, version, polyline)
         VALUES (?1, ?2, ?3)",
        params![activity_id, TRACK_VERSION, polyline],
    )?;
    Ok(())
}

/// Tracks of the activities of `view` matching `filter`, oldest first. Tracks not stored yet
/// are built from the archives without holding the database lock, in parallel.
pub fn load(
    db: &Db,
    imports_dir: &Path,
    view: ViewMode,
    filter: &ActivityFilter,
) -> AppResult<RouteHeatmap> {
    let rows = select(&*db.conn()?, view, filter)?;
    let missing: Vec<(i64, String)> = rows
        .iter()
        .filter(|r| r.polyline.is_none())
        .filter_map(|r| Some((r.id, r.file_id.clone()?)))
        .collect();
    let mut built = build(imports_dir, &missing);
    if !built.is_empty() {
        let mut conn = db.conn()?;
        let tx = conn.transaction()?;
        for (id, polyline) in &built {
            store(&tx, *id, polyline)?;
        }
        tx.commit()?;
    }
    let tracks = rows
        .into_iter()
        .filter_map(|r| r.polyline.or_else(|| built.remove(&r.id)))
        .filter(|p| !p.is_empty())
        .collect();
    Ok(RouteHeatmap { tracks })
}

struct Row {
    id: i64,
    file_id: Option<String>,
    /// `None` when not stored or stored by an older [`TRACK_VERSION`].
    polyline: Option<String>,
}

fn select(conn: &Connection, view: ViewMode, filter: &ActivityFilter) -> AppResult<Vec<Row>> {
    let mut sql = String::from(
        "SELECT activities.id, activities.file_id,
            CASE WHEN t.version = ? THEN t.polyline END
         FROM activities LEFT JOIN activity_tracks t ON t.activity_id = activities.id
         WHERE 1 = 1",
    );
    let mut values = vec![Value::Integer(TRACK_VERSION)];
    let types = view.allowed_types();
    if !types.is_empty() {
        sql.push_str(&format!(
            " AND activities.activity_type IN ({})",
            vec!["?"; types.len()].join(", ")
        ));
        values.extend(types.iter().map(|t| Value::Text((*t).to_string())));
    }
    filter.apply(&mut sql, &mut values);
    sql.push_str(" ORDER BY activities.date");
    Ok(conn
        .prepare(&sql)?
        .query_map(params_from_iter(values), |row| {
            Ok(Row {
                id: row.get(0)?,
                file_id: row.get(1)?,
                polyline: row.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?)
}

/// Encoded tracks read from the archives. Activities whose archive can't be read are left out
/// (and tried again next time); a track without GPS is stored empty so it isn't read again.
fn build(imports_dir: &Path, activities: &[(i64, String)]) -> HashMap<i64, String> {
    if activities.is_empty() {
        return HashMap::new();
    }
    let threads = thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = activities.len().div_ceil(threads);
    thread::scope(|scope| {
        let workers: Vec<_> = activities
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .filter_map(|(id, file_id)| {
                            let read = archive::read(&archive::archive_path(imports_dir, file_id))
                                .and_then(|xml| tcx::parse(&xml));
                            match read {
                                Ok(activity) => Some((*id, encode_track(&activity.points))),
                                Err(e) => {
                                    eprintln!("heat map track of activity {id} failed: {e}");
                                    None
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap_or_default())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_google_example() {
        assert_eq!(
            encode_polyline(&[(38.5, -120.2), (40.7, -120.95), (43.252, -126.453)]),
            "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
        );
    }

    #[test]
    fn drops_points_on_a_straight_line() {
        let time = chrono::Utc::now();
        let points: Vec<RawPoint> = (0..100)
            .map(|i| RawPoint {
                time,
                latitude: 48.0 + f64::from(i) * 1e-4,
                longitude: 11.0,
                elevation: 0.0,
                heart_rate: None,
                cadence: None,
                power: None,
            })
            .collect();
        assert_eq!(
            encode_track(&points),
            encode_polyline(&[(48.0, 11.0), (48.0099, 11.0)])
        );
        assert_eq!(encode_track(&points[..1]), "");
    }

    #[test]
    fn builds_missing_tracks_once() {
        use crate::{db::activities, importer};
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let dir = tempfile::tempdir().unwrap();
        let imports = dir.path().join("imports");
        let db = Db::open_in_memory().unwrap();
        let source = fixtures.join("synthetic_run.tcx");
        let (outcome, _) =
            importer::import_file(&mut db.conn().unwrap(), &source, &imports, "en").unwrap();
        let importer::ImportOutcome::Imported { activity_id } = outcome else {
            panic!("expected an import, got {outcome:?}");
        };
        let all = ActivityFilter::default();
        let stored = load(&db, &imports, ViewMode::All, &all).unwrap();
        assert_eq!(stored.tracks.len(), 1, "stored on import");
        assert!(!stored.tracks[0].is_empty());

        // A track missing from the table (an activity from before the heat map) is built from
        // the archive and stored; afterwards the archive isn't needed any more.
        db.conn()
            .unwrap()
            .execute("DELETE FROM activity_tracks", [])
            .unwrap();
        assert_eq!(load(&db, &imports, ViewMode::All, &all).unwrap(), stored);
        std::fs::remove_dir_all(&imports).unwrap();
        assert_eq!(
            load(&db, &imports, ViewMode::Running, &all).unwrap(),
            stored
        );

        assert!(
            load(&db, &imports, ViewMode::Cycling, &all)
                .unwrap()
                .tracks
                .is_empty()
        );
        let later = ActivityFilter {
            min_date: Some(i64::MAX - 1),
            ..Default::default()
        };
        assert!(
            load(&db, &imports, ViewMode::All, &later)
                .unwrap()
                .tracks
                .is_empty()
        );

        activities::delete(&mut db.conn().unwrap(), activity_id).unwrap();
        let left: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM activity_tracks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
    }
}
