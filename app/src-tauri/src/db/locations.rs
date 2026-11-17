//! Place names of activity starts (`activities.location`), looked up in the background.
//!
//! `location_latitude`/`location_longitude` hold the point that was looked up; they are set
//! even when no name was found, so each activity is looked up once. A recompute that moves the
//! start clears all three, and the activity is looked up again.

use rusqlite::{Connection, OptionalExtension, params};

use crate::{activity_type::ViewMode, error::AppResult};

/// A start this far (metres) from the looked-up point counts as moved.
const MOVED_METRES: f64 = 200.0;
/// A name found within this distance (metres) of a start is used for it without a request.
const NEARBY_METRES: f64 = 100.0;
const METRES_PER_DEGREE: f64 = 111_320.0;
/// Starts this close (metres) count as the same place for similar activities.
const SIMILAR_START_METRES: f64 = 200.0;
/// Share of the distance a similar activity may be off by.
const SIMILAR_DISTANCE_SHARE: f64 = 0.05;

/// An activity waiting for its place name, with its start point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate {
    pub activity_id: i64,
    pub latitude: f64,
    pub longitude: f64,
}

/// Condition on `activity_details` rows (alias `d`) with a usable position.
const HAS_GPS: &str = "d.seg_latitude IS NOT NULL AND d.seg_longitude IS NOT NULL
    AND (d.seg_latitude != 0 OR d.seg_longitude != 0)";

/// Start of an activity: its first segment with a position.
pub fn start_point(conn: &Connection, activity_id: i64) -> AppResult<Option<(f64, f64)>> {
    // `activity_details.activity_id` is a TEXT column in the Python schema.
    Ok(conn
        .query_row(
            &format!(
                "SELECT d.seg_latitude, d.seg_longitude FROM activity_details d
                 WHERE d.activity_id = ?1 AND {HAS_GPS}
                 ORDER BY d.segment_id LIMIT 1"
            ),
            [activity_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

/// The newest activity with GPS that hasn't been looked up yet.
pub fn next_candidate(conn: &Connection) -> AppResult<Option<Candidate>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT a.id, d.seg_latitude, d.seg_longitude FROM activities a
                 JOIN activity_details d ON d.activity_id = CAST(a.id AS TEXT)
                    AND d.segment_id = (
                        SELECT MIN(d.segment_id) FROM activity_details d
                        WHERE d.activity_id = CAST(a.id AS TEXT) AND {HAS_GPS})
                 WHERE a.location_latitude IS NULL
                 ORDER BY a.date DESC, a.id DESC LIMIT 1"
            ),
            [],
            |row| {
                Ok(Candidate {
                    activity_id: row.get(0)?,
                    latitude: row.get(1)?,
                    longitude: row.get(2)?,
                })
            },
        )
        .optional()?)
}

/// Approximate distance in metres, accurate enough over a few kilometres.
fn distance_metres(a: (f64, f64), b: (f64, f64)) -> f64 {
    let scale = ((a.0 + b.0) / 2.0).to_radians().cos();
    let dy = (a.0 - b.0) * METRES_PER_DEGREE;
    let dx = (a.1 - b.1) * METRES_PER_DEGREE * scale;
    dx.hypot(dy)
}

/// A place name already found for a start close to `candidate`, to save a request (most
/// activities start at the same few places).
pub fn nearby_name(conn: &Connection, candidate: &Candidate) -> AppResult<Option<String>> {
    let lat = NEARBY_METRES / METRES_PER_DEGREE;
    let lon = lat / candidate.latitude.to_radians().cos().max(0.01);
    let mut stmt = conn.prepare(
        "SELECT location, location_latitude, location_longitude FROM activities
         WHERE location IS NOT NULL AND location != '' AND id != ?1
           AND location_latitude BETWEEN ?2 AND ?3 AND location_longitude BETWEEN ?4 AND ?5",
    )?;
    let point = (candidate.latitude, candidate.longitude);
    let found = stmt
        .query_map(
            params![
                candidate.activity_id,
                point.0 - lat,
                point.0 + lat,
                point.1 - lon,
                point.1 + lon
            ],
            |row| Ok((row.get::<_, String>(0)?, (row.get(1)?, row.get(2)?))),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(found
        .into_iter()
        .map(|(name, at)| (distance_metres(point, at), name))
        .filter(|(distance, _)| *distance <= NEARBY_METRES)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, name)| name))
}

/// Stores the result for the looked-up point; `None` marks it as looked up without a name.
pub fn store(conn: &Connection, candidate: &Candidate, name: Option<&str>) -> AppResult<()> {
    conn.execute(
        "UPDATE activities SET location = ?2, location_latitude = ?3, location_longitude = ?4
         WHERE id = ?1",
        params![
            candidate.activity_id,
            name.filter(|n| !n.trim().is_empty()),
            candidate.latitude,
            candidate.longitude
        ],
    )?;
    Ok(())
}

/// Clears the place name when the start has moved away from (or lost) the looked-up point, so
/// it's looked up again. Returns whether it was cleared.
pub fn forget_if_moved(conn: &Connection, activity_id: i64) -> AppResult<bool> {
    let looked_up: Option<(Option<f64>, Option<f64>)> = conn
        .query_row(
            "SELECT location_latitude, location_longitude FROM activities WHERE id = ?1",
            [activity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((Some(lat), Some(lon))) = looked_up else {
        return Ok(false);
    };
    let moved = start_point(conn, activity_id)?
        .is_none_or(|start| distance_metres(start, (lat, lon)) > MOVED_METRES);
    if moved {
        conn.execute(
            "UPDATE activities SET location = NULL, location_latitude = NULL,
                location_longitude = NULL
             WHERE id = ?1",
            [activity_id],
        )?;
    }
    Ok(moved)
}

/// Where an activity starts: the looked-up point (kept in step with the start by
/// `forget_if_moved`), or else its first position.
fn known_start(
    conn: &Connection,
    activity_id: i64,
    looked_up: (Option<f64>, Option<f64>),
) -> AppResult<Option<(f64, f64)>> {
    match looked_up {
        (Some(lat), Some(lon)) => Ok(Some((lat, lon))),
        _ => start_point(conn, activity_id),
    }
}

/// Activities of the same sport as `activity_id` that start within 200 m of it and cover its
/// distance ±5 %, newest first, at most `limit`. Empty when it has no distance or no GPS.
pub fn similar(conn: &Connection, activity_id: i64, limit: usize) -> AppResult<Vec<i64>> {
    type Row = (i64, Option<String>, Option<f64>, Option<f64>, Option<f64>);
    let read = |row: &rusqlite::Row| -> rusqlite::Result<Row> {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    };
    let columns = "id, activity_type, distance, location_latitude, location_longitude";
    let Some((_, activity_type, distance, lat, lon)) = conn
        .query_row(
            &format!("SELECT {columns} FROM activities WHERE id = ?1"),
            [activity_id],
            read,
        )
        .optional()?
    else {
        return Ok(Vec::new());
    };
    let Some(distance) = distance.filter(|d| *d > 0.0) else {
        return Ok(Vec::new());
    };
    let Some(origin) = known_start(conn, activity_id, (lat, lon))? else {
        return Ok(Vec::new());
    };
    let activity_type = activity_type.unwrap_or_default();
    let view = ViewMode::from_activity_type(&activity_type);
    // Runs include trail runs and so on; types outside the views only match themselves.
    let same_sport = |other: &str| match view {
        ViewMode::All => other == activity_type,
        _ => ViewMode::from_activity_type(other) == view,
    };
    let candidates = conn
        .prepare(&format!(
            "SELECT {columns} FROM activities
             WHERE id != ?1 AND distance BETWEEN ?2 AND ?3
             ORDER BY date DESC, id DESC"
        ))?
        .query_map(
            params![
                activity_id,
                distance * (1.0 - SIMILAR_DISTANCE_SHARE),
                distance * (1.0 + SIMILAR_DISTANCE_SHARE)
            ],
            read,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let mut found = Vec::new();
    for (id, other_type, _, lat, lon) in candidates {
        if found.len() >= limit {
            break;
        }
        if !same_sport(other_type.as_deref().unwrap_or_default()) {
            continue;
        }
        if known_start(conn, id, (lat, lon))?
            .is_some_and(|start| distance_metres(origin, start) <= SIMILAR_START_METRES)
        {
            found.push(id);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    fn segment(conn: &Connection, activity_id: i64, segment_id: i64, position: Option<(f64, f64)>) {
        conn.execute(
            "INSERT INTO activity_details (activity_id, segment_id, seg_latitude, seg_longitude)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                activity_id.to_string(),
                segment_id,
                position.map(|p| p.0),
                position.map(|p| p.1)
            ],
        )
        .unwrap();
    }

    fn seeded() -> Db {
        let db = Db::open_in_memory().unwrap();
        {
            let conn = db.conn().unwrap();
            insert_activity(&conn, 1, "Running", 1_700_000_000, 10.0, "Old run");
            insert_activity(&conn, 2, "Running", 1_700_100_000, 5.0, "Treadmill");
            insert_activity(&conn, 3, "Running", 1_700_200_000, 8.0, "New run");
            segment(&conn, 1, 0, Some((47.98, 7.86)));
            // No GPS at all (zeros from the Python importer count as none).
            segment(&conn, 2, 0, None);
            segment(&conn, 2, 1, Some((0.0, 0.0)));
            // The first segment lost its position; the start is the second one's.
            segment(&conn, 3, 0, None);
            segment(&conn, 3, 1, Some((48.0, 7.9)));
        }
        db
    }

    #[test]
    fn picks_the_newest_with_gps_until_all_are_looked_up() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let first = next_candidate(&conn).unwrap().unwrap();
        assert_eq!(
            first,
            Candidate {
                activity_id: 3,
                latitude: 48.0,
                longitude: 7.9
            }
        );
        store(&conn, &first, Some("Freiburg im Breisgau, Wiehre")).unwrap();
        let second = next_candidate(&conn).unwrap().unwrap();
        assert_eq!(second.activity_id, 1);
        // Nothing found still counts as looked up.
        store(&conn, &second, None).unwrap();
        assert_eq!(next_candidate(&conn).unwrap(), None);

        let location: Option<String> = conn
            .query_row("SELECT location FROM activities WHERE id = 3", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(location.as_deref(), Some("Freiburg im Breisgau, Wiehre"));
    }

    #[test]
    fn reuses_names_found_close_by() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let first = next_candidate(&conn).unwrap().unwrap();
        store(&conn, &first, Some("Kirchzarten")).unwrap();
        let close = Candidate {
            activity_id: 1,
            latitude: 48.0005,
            longitude: 7.9005,
        };
        assert_eq!(
            nearby_name(&conn, &close).unwrap().as_deref(),
            Some("Kirchzarten")
        );
        let far = Candidate {
            latitude: 48.01,
            ..close
        };
        assert_eq!(nearby_name(&conn, &far).unwrap(), None);
    }

    #[test]
    fn forgets_the_name_when_the_start_moves() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let first = next_candidate(&conn).unwrap().unwrap();
        store(&conn, &first, Some("Kirchzarten")).unwrap();
        // A recompute with the same start keeps it.
        assert!(!forget_if_moved(&conn, 3).unwrap());
        assert_eq!(next_candidate(&conn).unwrap().unwrap().activity_id, 1);

        conn.execute(
            "UPDATE activity_details SET seg_latitude = 48.01 WHERE activity_id = '3'",
            [],
        )
        .unwrap();
        assert!(forget_if_moved(&conn, 3).unwrap());
        assert_eq!(next_candidate(&conn).unwrap().unwrap().activity_id, 3);
        // Never looked up: nothing to forget.
        assert!(!forget_if_moved(&conn, 2).unwrap());
    }

    #[test]
    fn measures_metres() {
        let d = distance_metres((48.0, 7.9), (48.001, 7.9));
        assert!((d - 111.3).abs() < 0.5, "{d}");
    }

    #[test]
    fn finds_similar_activities_by_sport_start_and_distance() {
        let db = seeded();
        let conn = db.conn().unwrap();
        // Activity 1: a 10 km run from (47.98, 7.86).
        insert_activity(&conn, 10, "Trailrun", 1_700_300_000, 10.3, "Same start");
        segment(&conn, 10, 0, Some((47.9805, 7.8605)));
        insert_activity(&conn, 11, "Running", 1_700_400_000, 10.0, "Looked up");
        segment(&conn, 11, 0, None);
        conn.execute(
            "UPDATE activities SET location_latitude = 47.9801, location_longitude = 7.8601
             WHERE id = 11",
            [],
        )
        .unwrap();
        insert_activity(&conn, 12, "Running", 1_700_500_000, 11.0, "Too long");
        segment(&conn, 12, 0, Some((47.98, 7.86)));
        insert_activity(&conn, 13, "Running", 1_700_600_000, 10.0, "Elsewhere");
        segment(&conn, 13, 0, Some((47.99, 7.86)));
        insert_activity(&conn, 14, "Biking", 1_700_700_000, 10.0, "A ride");
        segment(&conn, 14, 0, Some((47.98, 7.86)));
        insert_activity(&conn, 15, "Running", 1_700_800_000, 9.6, "No GPS");

        assert_eq!(similar(&conn, 1, 10).unwrap(), [11, 10]);
        assert_eq!(similar(&conn, 1, 1).unwrap(), [11]);
        // Without a start there is nothing to compare with.
        assert!(similar(&conn, 15, 10).unwrap().is_empty());
        assert!(similar(&conn, 99, 10).unwrap().is_empty());
    }
}
