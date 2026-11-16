//! GPX 1.0 / 1.1 reading (Garmin, Strava, Komoot, …). A GPX track becomes the same
//! [`TcxActivity`] a TCX file gives, and is archived as TCX (see [`super::tcx::write`]).

use chrono::{DateTime, Utc};
use roxmltree::{Document, Node};

use super::tcx::{RawSample, TcxActivity};
use crate::error::{AppError, AppResult};

fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

/// First element called `name` anywhere below `node` (extensions nest differently per app).
fn descendant<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

fn number(node: Option<Node>) -> Option<f64> {
    node?.text()?.trim().parse().ok()
}

/// TCX sport for a GPX `<type>`, which apps fill freely ("running", "Run", "cycling", "9", …).
/// Strava's old numeric codes: 1 ride, 9 run, 10 walk, 11 hike. Without a type it is a run.
fn sport(kind: Option<&str>) -> String {
    let Some(kind) = kind.map(|k| k.trim().to_ascii_lowercase()) else {
        return "Running".into();
    };
    let has = |words: &[&str]| words.iter().any(|w| kind.contains(w));
    if has(&["trail"]) {
        "Trailrun"
    } else if has(&["run", "jog", "lauf"]) || kind == "9" {
        "Running"
    } else if has(&["cycl", "bik", "ride", "rad", "velo"]) || kind == "1" {
        "Biking"
    } else if has(&["walk", "hik", "wander", "trek"]) || kind == "10" || kind == "11" {
        "Walking"
    } else if kind.is_empty() {
        "Running"
    } else {
        return kind;
    }
    .into()
}

/// Reads every `<trkpt>` with a time from all tracks. A missing `<ele>` takes the previous
/// altitude (0 before the first), so that every positioned point counts. Heart rate, cadence
/// and power come from Garmin's `TrackPointExtension` (`hr`, `cad`) and the common `power`
/// element.
pub fn parse(xml: &str) -> AppResult<TcxActivity> {
    let doc = Document::parse(xml).map_err(|e| AppError::Invalid(format!("invalid GPX: {e}")))?;
    let root = doc.root_element();
    if root.tag_name().name() != "gpx" {
        return Err(AppError::Invalid("not a GPX file".into()));
    }
    let kind = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "trk")
        .and_then(|trk| child(trk, "type"))
        .and_then(|n| n.text());

    let trackpoints: Vec<Node> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "trkpt")
        .collect();
    if trackpoints.is_empty() {
        return Err(AppError::Invalid("GPX without a track".into()));
    }

    let mut elevation = None;
    let samples: Vec<RawSample> = trackpoints
        .into_iter()
        .filter_map(|pt| {
            let time = child(pt, "time")?.text()?.trim();
            let time = DateTime::parse_from_rfc3339(time).ok()?.with_timezone(&Utc);
            let lat = pt.attribute("lat")?.trim().parse().ok()?;
            let lon = pt.attribute("lon")?.trim().parse().ok()?;
            elevation = number(child(pt, "ele")).or(elevation);
            let extensions = child(pt, "extensions");
            let extension = |name| extensions.and_then(|e| descendant(e, name));
            Some(RawSample {
                time,
                position: Some((lat, lon)),
                elevation: Some(elevation.unwrap_or(0.0)),
                heart_rate: number(extension("hr")),
                cadence: number(extension("cad")),
                power: number(extension("power")),
                distance: None,
            })
        })
        .collect();
    if samples.is_empty() {
        return Err(AppError::Invalid(
            "GPX without times: a planned route, not a recorded activity".into(),
        ));
    }
    Ok(TcxActivity::from_samples(sport(kind), samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GARMIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx creator="Garmin Connect" version="1.1" xmlns="http://www.topografix.com/GPX/1/1"
     xmlns:ns3="http://www.garmin.com/xmlschemas/TrackPointExtension/v1">
  <metadata><time>2025-05-06T11:00:00Z</time></metadata>
  <trk><name>Morning Run</name><type>running</type><trkseg>
    <trkpt lat="48.6" lon="-3.8"><ele>71.9</ele><time>2025-05-06T11:18:34Z</time>
      <extensions><ns3:TrackPointExtension><ns3:hr>120</ns3:hr><ns3:cad>79</ns3:cad></ns3:TrackPointExtension></extensions>
    </trkpt>
    <trkpt lat="48.61" lon="-3.81"><time>2025-05-06T11:18:36Z</time>
      <extensions><power>210</power></extensions>
    </trkpt>
  </trkseg></trk>
</gpx>"#;

    #[test]
    fn reads_garmin_gpx() {
        let activity = parse(GARMIN).unwrap();
        assert_eq!(activity.sport, "Running");
        assert_eq!(activity.points.len(), 2);
        let first = &activity.points[0];
        assert_eq!((first.latitude, first.longitude), (48.6, -3.8));
        assert_eq!(first.elevation, 71.9);
        assert_eq!(first.heart_rate, Some(120.0));
        assert_eq!(first.cadence, Some(79.0));
        assert_eq!(first.time.to_rfc3339(), "2025-05-06T11:18:34+00:00");
        // No <ele>: the previous altitude.
        assert_eq!(activity.points[1].elevation, 71.9);
        assert_eq!(activity.points[1].power, Some(210.0));
    }

    #[test]
    fn reads_gpx_1_0_without_extensions() {
        let xml = r#"<gpx version="1.0" xmlns="http://www.topografix.com/GPX/1/0"><trk><trkseg>
  <trkpt lat="52.5" lon="13.4"><time>2025-05-06T11:18:34.250Z</time></trkpt>
  <trkpt lat="52.6" lon="13.5"><ele>40</ele><time>2025-05-06T11:18:35Z</time></trkpt>
</trkseg></trk></gpx>"#;
        let activity = parse(xml).unwrap();
        assert_eq!(activity.sport, "Running");
        assert_eq!(activity.points[0].elevation, 0.0);
        assert_eq!(activity.points[1].elevation, 40.0);
        assert_eq!(activity.points[0].heart_rate, None);
    }

    #[test]
    fn maps_types_to_sports() {
        assert_eq!(sport(Some("Run")), "Running");
        assert_eq!(sport(Some("9")), "Running");
        assert_eq!(sport(Some("trail_running")), "Trailrun");
        assert_eq!(sport(Some("cycling")), "Biking");
        assert_eq!(sport(Some("Ride")), "Biking");
        assert_eq!(sport(Some("1")), "Biking");
        assert_eq!(sport(Some("hiking")), "Walking");
        assert_eq!(sport(Some("Wandern")), "Walking");
        assert_eq!(sport(None), "Running");
        assert_eq!(sport(Some("swimming")), "swimming");
    }

    #[test]
    fn rejects_routes_without_times() {
        let xml = r#"<gpx version="1.1" xmlns="http://www.topografix.com/GPX/1/1"><trk><trkseg>
  <trkpt lat="52.5" lon="13.4"><ele>30</ele></trkpt>
</trkseg></trk></gpx>"#;
        assert!(matches!(parse(xml), Err(AppError::Invalid(e)) if e.contains("without times")));
    }

    #[test]
    fn rejects_other_xml() {
        assert!(parse("<TrainingCenterDatabase/>").is_err());
        assert!(parse("<gpx").is_err());
    }
}
