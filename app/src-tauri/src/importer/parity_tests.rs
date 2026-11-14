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
/// none) and pauses (which Python under-counts in 1 s recordings).
fn assert_matches_python(p: &Processed, expected: &Value, check_heart_rate: bool) {
    let m = &p.metrics;
    assert_eq!(p.sport, expected["activity_type"].as_str().unwrap());
    close(m.start.timestamp() as f64, &expected["date"], 0.0, "date");
    close(m.distance, &expected["distance"], 0.011, "distance");
    close(m.duration as f64, &expected["duration"], 0.0, "duration");
    close(m.calories as f64, &expected["calories"], 1.0, "calories");
    close(
        m.elevation_gain as f64,
        &expected["elevation_gain"],
        0.0,
        "elevation_gain",
    );
    close(m.avg_speed, &expected["avg_speed"], 0.011, "avg_speed");
    close(m.avg_power as f64, &expected["avg_power"], 1.0, "avg_power");
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
    if let Some(total) = expected.get("total_steps") {
        close(m.total_steps as f64, total, 1.0, "total_steps");
        let avg = expected["avg_steps"]
            .as_f64()
            .map_or(0, |a| a.round_ties_even() as i64);
        assert_eq!(m.avg_steps, avg, "avg_steps");
    }

    let segments = expected["segments"].as_array().unwrap();
    assert_eq!(p.segments.len(), segments.len(), "segment count");
    for (i, (s, e)) in p.segments.iter().zip(segments).enumerate() {
        let what = |field: &str| format!("segment {i} {field}");
        close(s.distance, &e["seg_distance"], 1e-9, &what("distance"));
        close(s.avg_pace, &e["seg_avg_pace"], 1e-9, &what("pace"));
        close(
            s.elevation_gain,
            &e["seg_elevation_gain"],
            1e-9,
            &what("elevation_gain"),
        );
        close(s.avg_power as f64, &e["seg_avg_power"], 0.0, &what("power"));
        close(s.latitude, &e["seg_latitude"], 0.0, &what("latitude"));
        assert_eq!(
            segments::format_time(s.time_start),
            e["seg_time_start"].as_str().unwrap(),
            "{}",
            what("start")
        );
        assert_eq!(
            segments::format_time(s.time_end),
            e["seg_time_end"].as_str().unwrap(),
            "{}",
            what("end")
        );
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

    let best = segments::best_segments(&p.segments, performances::segment_distances(p.view));
    let expected_best = expected["best"].as_object().unwrap();
    assert_eq!(best.len(), expected_best.len(), "best segment count");
    for b in best {
        close(
            b.pace,
            &expected_best[&(b.distance as i64).to_string()],
            1e-9,
            "best pace",
        );
    }
}

#[test]
fn synthetic_run_matches_python() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let xml = std::fs::read_to_string(fixtures.join("synthetic_run.tcx")).unwrap();
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures.join("synthetic_run.expected.json")).unwrap(),
    )
    .unwrap();
    let processed = process("synthetic_run", &xml).unwrap();
    assert_matches_python(&processed, &expected, true);

    // The fixture has one 45 s gap. Python reports no pause because it smooths 1 s recordings
    // before detecting pauses.
    assert_eq!(expected["pause_minutes"].as_f64(), Some(0.0));
    assert!((processed.metrics.pause_minutes - 0.75).abs() < 1e-9);
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
        let processed = process(name.trim_end_matches(".tcx"), &xml).unwrap();
        // Heart rate only matches Python when every trackpoint with a position has one.
        let points = tcx::parse(&xml).unwrap().points.len();
        let aligned = xml.matches("<HeartRateBpm").count() == points;
        assert_matches_python(&processed, values, aligned);
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
    assert_eq!(detail.total_steps, Some(3045));
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

    // Re-import keeps the user's title and doesn't duplicate best performances.
    conn.execute(
        "UPDATE activities SET title = 'My run' WHERE id = ?1",
        [activity_id],
    )
    .unwrap();
    reimport(&mut conn, activity_id, &imports, "en").unwrap();
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
