//! Checks the port against values computed by the Python importer for the same file. The
//! expected JSON is produced by running the Python pipeline (`TcxFileParser`,
//! `TcxSegmentParser`, `data_processing`, `compute_data`, `BestSegmentFinder`).

use std::path::Path;

use serde_json::Value;

use super::*;

fn close(actual: f64, expected: &Value, tolerance: f64, what: &str) {
    let expected = expected
        .as_f64()
        .unwrap_or_else(|| panic!("{what}: expected a number, got {expected}"));
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: {actual} vs Python {expected}"
    );
}

/// Compares everything except heart rate (which Python misaligns when some trackpoints have
/// none), pauses (which Python under-counts in 1 s recordings), total steps (which Python
/// counted through pauses), calories and power (Python's running and walking estimate for points
/// without measured watts was about five times too low).
fn assert_matches_python(
    p: &Processed,
    expected: &Value,
    check_heart_rate: bool,
    cuts_match: bool,
) {
    let m = &p.metrics;
    assert_eq!(p.sport, expected["activity_type"].as_str().unwrap());
    close(m.start.timestamp() as f64, &expected["date"], 0.0, "date");
    close(m.distance, &expected["distance"], 0.011, "distance");
    close(m.duration as f64, &expected["duration"], 0.0, "duration");
    close(
        m.elevation_gain as f64,
        &expected["elevation_gain"],
        0.0,
        "elevation_gain",
    );
    close(m.avg_speed, &expected["avg_speed"], 0.011, "avg_speed");
    close(m.avg_pace, &expected["avg_pace"], 1e-6, "avg_pace");
    close(
        m.fastest_pace,
        &expected["fastest_pace"],
        1e-6,
        "fastest_pace",
    );
    close(
        m.slowest_pace,
        &expected["slowest_pace"],
        1e-6,
        "slowest_pace",
    );
    assert_eq!(
        metrics::format_min_sec(m.avg_pace),
        expected["avg_pace_text"].as_str().unwrap()
    );
    if check_heart_rate {
        close(
            m.avg_heart_rate as f64,
            &expected["avg_heart_rate"],
            0.0,
            "avg_heart_rate",
        );
    }
    // Total steps aren't compared: Python counted the cadence through pauses.
    if expected.get("total_steps").is_some() {
        let avg = expected["avg_steps"]
            .as_f64()
            .map_or(0, |a| a.round_ties_even() as i64);
        assert_eq!(m.avg_steps, avg, "avg_steps");
    }

    // Segments start where the previous one ended (Python: at the point after the cut, which
    // dropped that step's time), and are cut on the raw distance, which only matches Python's
    // cuts in tracks without standing still.
    if !cuts_match {
        return;
    }
    let segments = expected["segments"].as_array().unwrap();
    assert_eq!(p.segments.len(), segments.len(), "segment count");
    for (i, (s, e)) in p.segments.iter().zip(segments).enumerate() {
        let what = |field: &str| format!("segment {i} {field}");
        close(s.distance, &e["seg_distance"], 1e-9, &what("distance"));
        close(
            s.elevation_gain,
            &e["seg_elevation_gain"],
            1e-9,
            &what("elevation_gain"),
        );
        assert_eq!(
            segments::format_time(s.time_end),
            e["seg_time_end"].as_str().unwrap(),
            "{}",
            what("end")
        );
        if i == 0 {
            close(s.avg_pace, &e["seg_avg_pace"], 1e-9, &what("pace"));
            close(s.latitude, &e["seg_latitude"], 0.0, &what("latitude"));
            assert_eq!(
                segments::format_time(s.time_start),
                e["seg_time_start"].as_str().unwrap(),
                "{}",
                what("start")
            );
        } else {
            assert_eq!(
                s.time_start,
                p.segments[i - 1].time_end,
                "{}",
                what("start")
            );
            let minutes = (s.time_end - s.time_start).as_seconds_f64() / 60.0;
            assert!(
                (s.avg_pace - minutes / s.distance).abs() < 1e-9,
                "{}",
                what("pace")
            );
        }
        match (s.avg_speed, e["seg_avg_speed"].as_f64()) {
            (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{}", what("speed")),
            (a, b) => assert_eq!(a, b, "{}", what("speed")),
        }
        if check_heart_rate {
            close(
                s.avg_heart_rate as f64,
                &e["seg_avg_heart_rate"],
                0.0,
                &what("heart rate"),
            );
        }
    }

    // Best paces are time over distance rather than Python's mean of segment paces, which stays
    // within a few seconds per km for full-length segments.
    let best = segments::best_segments(&p.segments, performances::segment_distances(p.view));
    let expected_best = expected["best"].as_object().unwrap();
    assert_eq!(best.len(), expected_best.len(), "best segment count");
    for b in best {
        close(
            b.pace,
            &expected_best[&(b.distance as i64).to_string()],
            0.05,
            "best pace",
        );
    }
}

/// Whether no two consecutive trackpoints share a position, so the segment cuts are Python's.
fn cuts_match(xml: &str) -> bool {
    tcx::parse(xml)
        .unwrap()
        .points
        .windows(2)
        .all(|w| (w[0].latitude, w[0].longitude) != (w[1].latitude, w[1].longitude))
}

#[test]
fn synthetic_run_matches_python() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let xml = std::fs::read_to_string(fixtures.join("synthetic_run.tcx")).unwrap();
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures.join("synthetic_run.expected.json")).unwrap(),
    )
    .unwrap();
    let processed = process("synthetic_run", &xml, track::DEFAULT_BODY_WEIGHT).unwrap();
    assert!(cuts_match(&xml), "the fixture has no standing still");
    assert_matches_python(&processed, &expected, true, true);

    // The fixture has one 45 s gap. Python reports no pause because it smooths 1 s recordings
    // before detecting pauses.
    assert_eq!(expected["pause_minutes"].as_f64(), Some(0.0));
    assert!((processed.metrics.pause_minutes - 0.75).abs() < 1e-9);
    // Python also counted 45 s of steps through it, at 156 steps per minute.
    assert_eq!(
        expected["total_steps"].as_f64().map(f64::round),
        Some(3045.0)
    );
    assert_eq!(processed.metrics.total_steps, 3045 - 117);
}

/// Opt-in check against real recordings:
/// `RUNNINGMATE_GOLDEN_DIR=/dir cargo test real_files -- --ignored`, where the directory holds
/// `.tcx` files and an `expected.json` keyed by file name.
#[test]
#[ignore = "needs RUNNINGMATE_GOLDEN_DIR with TCX files and Python reference values"]
fn real_files_match_python() {
    let dir = std::env::var("RUNNINGMATE_GOLDEN_DIR").expect("RUNNINGMATE_GOLDEN_DIR");
    let dir = Path::new(&dir);
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("expected.json")).unwrap()).unwrap();
    for (name, values) in expected.as_object().unwrap() {
        let xml = std::fs::read_to_string(dir.join(name)).unwrap();
        let processed = process(
            name.trim_end_matches(".tcx"),
            &xml,
            track::DEFAULT_BODY_WEIGHT,
        )
        .unwrap();
        // Heart rate only matches Python when every trackpoint with a position has one.
        let points = tcx::parse(&xml).unwrap().points.len();
        let aligned = xml.matches("<HeartRateBpm").count() == points;
        assert_matches_python(&processed, values, aligned, cuts_match(&xml));
        println!(
            "{name}: ok ({} segments, heart rate {})",
            processed.segments.len(),
            if aligned {
                "compared"
            } else {
                "skipped: Python misaligns it"
            }
        );
    }
}

#[test]
fn import_stores_activity_and_reimport_keeps_edits() {
    use crate::db::{Db, activities, performances};

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let mut conn = db.conn().unwrap();

    let (outcome, weather) = import_file(&mut conn, &source, &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    assert!(weather.is_some());
    assert!(source.exists(), "the source file is left in place");
    assert!(archive::archive_path(&imports, "synthetic_run").exists());

    let detail = activities::get(&conn, activity_id).unwrap();
    assert_eq!(detail.view_mode, ViewMode::Running);
    assert_eq!(detail.distance, Some(3.21));
    assert_eq!(detail.avg_pace.as_deref(), Some("05:44"));
    assert_eq!(detail.pause.as_deref(), Some("00:45"));
    assert_eq!(detail.total_steps, Some(2928));
    assert!(detail.is_new);
    let segments: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM activity_details WHERE activity_id = ?1",
            [activity_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(segments, 4);
    assert_eq!(
        performances::for_activity(&conn, activity_id)
            .unwrap()
            .len(),
        1
    );

    // Same file again: reported as duplicate, nothing added.
    let (again, _) = import_file(&mut conn, &source, &imports, "en").unwrap();
    assert_eq!(again, ImportOutcome::Duplicate { activity_id });

    // Re-import keeps the user's title and doesn't duplicate best performances. It restores the
    // distance changed to 5 km, and the shoe follows.
    conn.execute(
        "UPDATE activities SET title = 'My run', distance = 5 WHERE id = ?1",
        [activity_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO shoes (id, name, distance, status) VALUES (7, 'Racer', 100, 1)",
        [],
    )
    .unwrap();
    activities::set_equipment(&mut conn, activity_id, Some(7)).unwrap();
    reimport(&mut conn, activity_id, &imports, "en").unwrap();
    let shoe: f64 = conn
        .query_row("SELECT distance FROM shoes WHERE id = 7", [], |r| r.get(0))
        .unwrap();
    assert!((shoe - (100.0 + 3.21)).abs() < 1e-9, "{shoe}");
    assert_eq!(
        activities::get(&conn, activity_id)
            .unwrap()
            .title
            .as_deref(),
        Some("My run")
    );
    assert_eq!(
        performances::for_activity(&conn, activity_id)
            .unwrap()
            .len(),
        1
    );
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM best_performances WHERE activity_id = ?1",
            [activity_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(rows, 1);

    let outline = route_outline(&conn, activity_id, &imports)
        .unwrap()
        .unwrap();
    assert!(outline.points.len() > 10 && outline.points.len() < 1100);
}

#[test]
fn same_day_and_distance_counts_as_similar() {
    use crate::db::{Db, activities::tests::insert_activity};

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let db = Db::open_in_memory().unwrap();
    let mut conn = db.conn().unwrap();

    // The same run, recorded by another app: same day, 3.23 km instead of 3.21 km.
    let start = file_start(&source).unwrap();
    insert_activity(
        &conn,
        77,
        "Running",
        start.timestamp() + 600,
        3.23,
        "Other app",
    );
    let (outcome, weather) = import_file(&mut conn, &source, dir.path(), "en").unwrap();
    assert_eq!(outcome, ImportOutcome::Similar { activity_id: 77 });
    assert!(weather.is_none());

    // A clearly different distance on the same day is imported.
    conn.execute("UPDATE activities SET distance = 5.0 WHERE id = 77", [])
        .unwrap();
    let (outcome, _) = import_file(&mut conn, &source, dir.path(), "en").unwrap();
    assert!(matches!(outcome, ImportOutcome::Imported { .. }));
}

#[test]
fn tolerance_is_one_percent_or_fifty_metres() {
    assert!(same_distance(10.0, 10.09));
    assert!(!same_distance(10.0, 10.2));
    assert!(same_distance(2.0, 2.05));
    assert!(!same_distance(2.0, 2.06));
}

#[test]
fn energy_is_recomputed_once_and_keeps_edits() {
    use crate::db::Db;

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let (outcome, _) = import_file(&mut db.conn().unwrap(), &source, &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    let energy = || -> (i64, i64, f64, String) {
        db.conn()
            .unwrap()
            .query_row(
                "SELECT a.calories, r.avg_power, d.seg_avg_power, a.title
                 FROM activities a JOIN runs r ON r.activity_id = a.id
                 JOIN activity_details d ON d.activity_id = a.id AND d.segment_id = 0
                 WHERE a.id = ?1",
                [activity_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap()
    };
    let imported = energy();
    let spoil = || {
        db.conn()
            .unwrap()
            .execute_batch(
                "UPDATE activities SET calories = 1, title = 'My run';
                 UPDATE runs SET avg_power = 1;
                 UPDATE activity_details SET seg_avg_power = 1;",
            )
            .unwrap();
    };

    spoil();
    apply_data_fixes(&db, &imports, "en").unwrap();
    let (calories, power, segment_power, title) = energy();
    assert_eq!(
        (calories, power, segment_power),
        (imported.0, imported.1, imported.2)
    );
    assert_eq!(title, "My run");

    spoil();
    apply_data_fixes(&db, &imports, "en").unwrap();
    assert_eq!(energy().0, 1, "runs only once");
}

#[test]
fn grade_adjusted_pace_is_stored_and_filled_in_once() {
    use crate::db::{Db, activities};

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let (outcome, _) = import_file(&mut db.conn().unwrap(), &source, &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    let stored = || {
        let conn = db.conn().unwrap();
        let detail = activities::get(&conn, activity_id).unwrap();
        let splits = crate::analysis::segments(&conn, activity_id).unwrap();
        (
            detail.avg_gap,
            splits.iter().map(|s| s.gap).collect::<Vec<_>>(),
        )
    };
    let imported = stored();
    assert!(
        imported.0.as_deref().is_some_and(|g| g.len() == 5),
        "{imported:?}"
    );
    assert_eq!(imported.1.len(), 4);
    assert!(imported.1.iter().all(|g| g.is_some_and(|g| g > 0.0)));

    // As stored before the grade-adjusted pace was computed.
    let forget = || {
        db.conn()
            .unwrap()
            .execute_batch(
                "UPDATE runs SET avg_gap = NULL;
                 UPDATE activity_details SET seg_avg_gap = NULL;",
            )
            .unwrap();
    };
    forget();
    apply_data_fixes(&db, &imports, "en").unwrap();
    assert_eq!(stored(), imported);

    forget();
    apply_data_fixes(&db, &imports, "en").unwrap();
    assert_eq!(stored().0, None, "runs only once");
}

#[test]
fn pauses_and_segments_are_recomputed_once_and_keep_edits() {
    use crate::db::{Db, activities};

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let (outcome, _) = import_file(&mut db.conn().unwrap(), &source, &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    let stored = || -> (Option<i64>, i64, i64, String) {
        let conn = db.conn().unwrap();
        let detail = activities::get(&conn, activity_id).unwrap();
        let (segments, bests) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM activity_details WHERE activity_id = ?1),
                        (SELECT COUNT(*) FROM best_performances WHERE activity_id = ?1)",
                [activity_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        (detail.total_steps, segments, bests, detail.title.unwrap())
    };
    let imported = stored();
    // As an earlier version left it, plus the user's edits.
    let spoil = || {
        db.conn()
            .unwrap()
            .execute_batch(
                "UPDATE activities SET title = 'My run', distance = 3.3;
                 UPDATE runs SET total_steps = 1;
                 DELETE FROM activity_details WHERE segment_id > 0;
                 DELETE FROM best_performances;",
            )
            .unwrap();
    };

    spoil();
    apply_data_fixes(&db, &imports, "en").unwrap();
    let (steps, segments, bests, title) = stored();
    assert_eq!(
        (steps, segments, bests),
        (imported.0, imported.1, imported.2)
    );
    assert_eq!(title, "My run");
    let distance = activities::get(&db.conn().unwrap(), activity_id)
        .unwrap()
        .distance;
    assert_eq!(distance, Some(3.3));

    spoil();
    apply_data_fixes(&db, &imports, "en").unwrap();
    assert_eq!(stored().0, Some(1), "runs only once");
}

#[test]
fn clock_jumps_are_fixed_in_stored_activities_not_edited() {
    use crate::db::Db;

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let (outcome, _) = import_file(&mut db.conn().unwrap(), &source, &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    // As Garmin exported it: a first point without GPS two weeks before all the others.
    let xml = std::fs::read_to_string(&source).unwrap().replacen(
        "<Trackpoint>",
        "<Trackpoint><Time>2025-02-15T07:30:00Z</Time></Trackpoint><Trackpoint>",
        1,
    );
    archive::write(&imports, "synthetic_run", &xml).unwrap();
    let date = || -> i64 {
        db.conn()
            .unwrap()
            .query_row(
                "SELECT date FROM activities WHERE id = ?1",
                [activity_id],
                |r| r.get(0),
            )
            .unwrap()
    };
    let stored = date();

    let set_edited = |edited: bool| {
        db.conn()
            .unwrap()
            .execute(
                "UPDATE activities SET edited = ?1 WHERE id = ?2",
                rusqlite::params![edited, activity_id],
            )
            .unwrap();
    };
    set_edited(true);
    assert!(fix_clock_jumps(&db, &imports, "en").unwrap().is_empty());
    assert_eq!(date(), stored, "edited activities are left alone");

    set_edited(false);
    let weather = fix_clock_jumps(&db, &imports, "en").unwrap();
    assert_eq!(weather.len(), 1);
    assert_eq!(date(), stored - 14 * 24 * 3600);
}

#[test]
fn new_route_segment_is_matched_in_archived_activities() {
    use crate::db::{Db, route_segments};

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let (outcome, _) = import_file(&mut db.conn().unwrap(), &source, &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    let track = crate::analysis::load_track(&db.conn().unwrap(), activity_id, &imports)
        .unwrap()
        .unwrap();
    let total = track.distance_km[track.len() - 1];
    let shape = segment_match::cut(&track, total * 0.3, total * 0.7).unwrap();
    let segment = route_segments::create(
        &db.conn().unwrap(),
        "Middle",
        track.view,
        activity_id,
        &shape,
        0,
    )
    .unwrap();

    assert_eq!(match_segment(&db, &imports, segment).unwrap(), 1);
    // Matching again replaces the passes instead of adding to them.
    assert_eq!(match_segment(&db, &imports, segment).unwrap(), 1);
    let detail = route_segments::get(&db.conn().unwrap(), segment).unwrap();
    assert_eq!(detail.leaderboard.len(), 1);
    assert_eq!(detail.leaderboard[0].activity_id, activity_id);
    assert_eq!(detail.leaderboard[0].rank, 1);
}

#[test]
fn gpx_import_matches_the_same_run_as_tcx() {
    use crate::db::{Db, activities};

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.gpx");
    std::fs::copy(fixtures.join("synthetic_run.gpx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let mut conn = db.conn().unwrap();

    assert_eq!(
        file_start(&source).unwrap().to_rfc3339(),
        "2025-03-01T07:30:01+00:00",
        "the first trackpoint, not the metadata time"
    );
    let (outcome, _) = import_file(&mut conn, &source, &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    let detail = activities::get(&conn, activity_id).unwrap();
    assert_eq!(detail.view_mode, ViewMode::Running);
    assert_eq!(detail.distance, Some(3.21));
    assert_eq!(detail.avg_pace.as_deref(), Some("05:44"));
    assert_eq!(detail.total_steps, Some(2928));

    // Archived as TCX, so re-import and analysis read it like any other activity.
    let xml = archive::read(&archive::archive_path(&imports, "synthetic_run")).unwrap();
    assert_eq!(tcx::parse(&xml).unwrap().points.len(), 1100);
    reimport(&mut conn, activity_id, &imports, "en").unwrap();
    assert_eq!(
        activities::get(&conn, activity_id).unwrap().distance,
        Some(3.21)
    );
}

#[test]
fn fixing_a_route_recomputes_the_activity_and_reverts() {
    use crate::db::{Db, activities};
    use repair::FixEdit;

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic_run.tcx");
    std::fs::copy(fixtures.join("synthetic_run.tcx"), &source).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let mut conn = db.conn().unwrap();
    let (ImportOutcome::Imported { activity_id }, _) =
        import_file(&mut conn, &source, &imports, "en").unwrap()
    else {
        panic!("expected an import");
    };
    conn.execute(
        "UPDATE activities SET title = 'My run' WHERE id = ?1",
        [activity_id],
    )
    .unwrap();

    let found = fix_source(&conn, activity_id, &imports).unwrap();
    assert!(!found.has_original);
    let samples = &found.samples;
    // Everything from sample 100 on, drawn as a detour of a detour about 1 km east.
    let (a, b) = (&samples[100], &samples[samples.len() - 1]);
    let start = (a.lat.unwrap(), a.lon.unwrap());
    let path = vec![
        start,
        (start.0, start.1 + 0.0135),
        (b.lat.unwrap(), b.lon.unwrap()),
    ];
    let metres = |a: (f64, f64), b: (f64, f64)| -> f64 {
        geographiclib_rs::InverseGeodesic::inverse(
            &geographiclib_rs::Geodesic::wgs84(),
            a.0,
            a.1,
            b.0,
            b.1,
        )
    };
    let before: f64 = samples[..=100]
        .windows(2)
        .map(|w| {
            metres(
                (w[0].lat.unwrap(), w[0].lon.unwrap()),
                (w[1].lat.unwrap(), w[1].lon.unwrap()),
            )
        })
        .sum();
    let drawn: f64 = path.windows(2).map(|w| metres(w[0], w[1])).sum();
    let edit = FixEdit {
        from: a.time,
        to: b.time,
        path,
    };
    fix_activity(&mut conn, activity_id, &[edit], &imports, "en").unwrap();
    let fixed = activities::get(&conn, activity_id).unwrap();
    let expected = (before + drawn) / 1000.0;
    let distance = fixed.distance.unwrap();
    assert!(
        (distance - expected).abs() < 0.02,
        "{distance} vs {expected}"
    );
    assert_eq!(fixed.title.as_deref(), Some("My run"));
    assert!(
        fix_source(&conn, activity_id, &imports)
            .unwrap()
            .has_original
    );

    revert_fix(&mut conn, activity_id, &imports, "en").unwrap();
    assert_eq!(
        activities::get(&conn, activity_id).unwrap().distance,
        Some(3.21)
    );
    assert!(
        !fix_source(&conn, activity_id, &imports)
            .unwrap()
            .has_original
    );
}

#[test]
fn a_file_without_gps_waits_for_its_drawn_route() {
    use crate::db::{Db, activities};
    use repair::FixEdit;

    // Ten minutes on a treadmill: times, heart rate and the foot pod's distance, no GPS.
    let points: String = (0..=600)
        .map(|s| {
            format!(
                "<Trackpoint><Time>{}</Time><DistanceMeters>{}</DistanceMeters><HeartRateBpm><Value>140</Value></HeartRateBpm></Trackpoint>",
                (chrono::DateTime::from_timestamp(1_746_522_000 + s, 0).unwrap()).to_rfc3339(),
                s * 3
            )
        })
        .collect();
    let xml = format!(
        r#"<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2"><Activities><Activity Sport="Running"><Lap><Track>{points}</Track></Lap></Activity></Activities></TrainingCenterDatabase>"#
    );
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("treadmill.tcx");
    std::fs::write(&source, xml).unwrap();
    let imports = dir.path().join("imports");
    let db = Db::open_in_memory().unwrap();
    let mut conn = db.conn().unwrap();

    let (outcome, _) = import_file(&mut conn, &source, &imports, "en").unwrap();
    assert_eq!(
        outcome,
        ImportOutcome::NeedsRoute {
            file_id: "treadmill".into()
        }
    );
    let waiting = pending::list(&imports).unwrap();
    assert_eq!(waiting.len(), 1);
    assert_eq!(waiting[0].distance, Some(1.8));
    assert_eq!(waiting[0].duration, 600.0);
    assert!(pending::source(&conn, &imports, "../treadmill").is_err());

    let found = pending::source(&conn, &imports, "treadmill").unwrap();
    assert_eq!(found.issues[0].kind, repair::FixIssueKind::NoGps);
    // 1.8 km north.
    let edit = FixEdit {
        from: found.samples[0].time,
        to: found.samples[600].time,
        path: vec![(48.0, 2.0), (48.0 + 1800.0 / 111_200.0, 2.0)],
    };
    let (outcome, _) = pending::import(&mut conn, "treadmill", &[edit], &imports, "en").unwrap();
    let ImportOutcome::Imported { activity_id } = outcome else {
        panic!("expected an import, got {outcome:?}");
    };
    let detail = activities::get(&conn, activity_id).unwrap();
    assert!(
        (detail.distance.unwrap() - 1.8).abs() < 0.02,
        "{:?}",
        detail.distance
    );
    assert!(pending::list(&imports).unwrap().is_empty());
    assert!(archive::original_path(&imports, "treadmill").exists());

    // The original has no route to go back to.
    assert!(
        !fix_source(&conn, activity_id, &imports)
            .unwrap()
            .has_original
    );
    assert!(revert_fix(&mut conn, activity_id, &imports, "en").is_err());
    assert!(archive::archive_path(&imports, "treadmill").exists());
}
