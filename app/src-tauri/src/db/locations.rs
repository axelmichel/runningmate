//! Place names of activity starts (`activities.location`), looked up in the background.
//!
//! `location_latitude`/`location_longitude` hold the point that was looked up; they are set
//! even when no name was found, so each activity is looked up once. A recompute that moves the
//! start clears all three, and the activity is looked up again.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::AppResult;

/// A start this far (metres) from the looked-up point counts as moved.
const MOVED_METRES: f64 = 200.0;
/// A name found within this distance (metres) of a start is used for it without a request.
const NEARBY_METRES: f64 = 100.0;
const METRES_PER_DEGREE: f64 = 111_320.0;

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
}
