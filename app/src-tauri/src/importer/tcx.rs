//! TCX (Garmin Training Center XML) reading, the XML part of `processing/tcx_file_parser.py`.

use chrono::{DateTime, TimeDelta, Utc};
use roxmltree::{Document, Node};

use crate::error::{AppError, AppResult};

const ACTIVITY_EXTENSION_NS: &str = "http://www.garmin.com/xmlschemas/ActivityExtension/v2";

/// One `<Trackpoint>` with a position, altitude and time.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPoint {
    pub time: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f64,
    pub heart_rate: Option<f64>,
    /// `RunCadence` from the activity extension: steps per minute of one foot.
    pub cadence: Option<f64>,
    /// `Watts` from the activity extension.
    pub power: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TcxActivity {
    /// `Sport` attribute of the first `<Activity>` ("Running", "Biking", "Other", …).
    pub sport: String,
    pub points: Vec<RawPoint>,
    /// Clock jumps closed by [`without_clock_jumps`].
    pub clock_jumps: usize,
}

fn child<'a>(node: Node<'a, 'a>, name: &str) -> Option<Node<'a, 'a>> {
    node.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}

fn number(node: Node, name: &str) -> Option<f64> {
    child(node, name)?.text()?.trim().parse().ok()
}

/// A gap between two trackpoints longer than this is a clock jump, not a pause.
const CLOCK_JUMP: TimeDelta = TimeDelta::hours(12);

/// Closes clock jumps: every time after a gap longer than [`CLOCK_JUMP`] moves back so that it
/// follows the previous one by a second, keeping the spacing of the times after it. Garmin
/// exported a run from 31 October whose first trackpoints (no GPS yet) were dated correctly and
/// all later ones two weeks too late, which dated the whole activity to the later day.
fn without_clock_jumps(mut times: Vec<DateTime<Utc>>) -> (Vec<DateTime<Utc>>, usize) {
    let mut shift = TimeDelta::zero();
    let mut jumps = 0;
    for i in 1..times.len() {
        let original = times[i];
        let gap = original - shift - times[i - 1];
        if gap > CLOCK_JUMP {
            shift += gap - TimeDelta::seconds(1);
            jumps += 1;
        }
        times[i] = original - shift;
    }
    (times, jumps)
}

/// Reads all trackpoints. Like the Python parser, points without position, altitude or time are
/// skipped. Unlike it, heart rate stays with its own trackpoint: Python collected heart rates
/// into a separate list and lined them up by index, which shifted them whenever some points had
/// no heart rate. Clock jumps are removed first, across all trackpoints, so that points without
/// a position still anchor the start (see [`without_clock_jumps`]).
pub fn parse(xml: &str) -> AppResult<TcxActivity> {
    let doc = Document::parse(xml).map_err(|e| AppError::Invalid(format!("invalid TCX: {e}")))?;
    let root = doc.root_element();

    let sport = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Activity")
        .and_then(|n| n.attribute("Sport"))
        .unwrap_or("Unknown")
        .to_string();

    let trackpoints: Vec<(Node, DateTime<Utc>)> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Trackpoint")
        .filter_map(|tp| {
            let time = child(tp, "Time")?.text()?.trim();
            let time = DateTime::parse_from_rfc3339(time).ok()?.with_timezone(&Utc);
            Some((tp, time))
        })
        .collect();
    let (times, clock_jumps) = without_clock_jumps(trackpoints.iter().map(|(_, t)| *t).collect());

    let points = trackpoints
        .iter()
        .zip(times)
        .filter_map(|(&(tp, _), time)| {
            let extension = tp.descendants().find(|n| {
                n.is_element()
                    && n.tag_name().name() == "TPX"
                    && n.tag_name().namespace() == Some(ACTIVITY_EXTENSION_NS)
            });
            Some(RawPoint {
                time,
                latitude: number(tp, "LatitudeDegrees")?,
                longitude: number(tp, "LongitudeDegrees")?,
                elevation: number(tp, "AltitudeMeters")?,
                heart_rate: child(tp, "HeartRateBpm").and_then(|hr| number(hr, "Value")),
                cadence: extension.and_then(|e| number(e, "RunCadence")),
                power: extension.and_then(|e| number(e, "Watts")),
            })
        })
        .collect();

    Ok(TcxActivity {
        sport,
        points,
        clock_jumps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities><Activity Sport="Running"><Lap><Track>
    <Trackpoint><Time>2025-05-06T11:18:34Z</Time>
      <Position><LatitudeDegrees>48.6</LatitudeDegrees><LongitudeDegrees>-3.8</LongitudeDegrees></Position>
      <AltitudeMeters>71.9</AltitudeMeters>
      <Extensions><TPX xmlns="http://www.garmin.com/xmlschemas/ActivityExtension/v2"><Watts>142</Watts><RunCadence>79</RunCadence></TPX></Extensions>
    </Trackpoint>
    <Trackpoint><Time>2025-05-06T11:18:35.500Z</Time><AltitudeMeters>72</AltitudeMeters>
      <HeartRateBpm><Value>120</Value></HeartRateBpm></Trackpoint>
    <Trackpoint><Time>2025-05-06T11:18:36Z</Time>
      <Position><LatitudeDegrees>48.61</LatitudeDegrees><LongitudeDegrees>-3.81</LongitudeDegrees></Position>
      <AltitudeMeters>72.5</AltitudeMeters><HeartRateBpm><Value>129</Value></HeartRateBpm>
    </Trackpoint>
  </Track></Lap></Activity></Activities>
</TrainingCenterDatabase>"#;

    #[test]
    fn keeps_heart_rate_with_its_trackpoint() {
        let activity = parse(SAMPLE).unwrap();
        assert_eq!(activity.sport, "Running");
        // The middle point has no position and is skipped; its heart rate must not move.
        assert_eq!(activity.points.len(), 2);
        assert_eq!(activity.points[0].heart_rate, None);
        assert_eq!(activity.points[0].power, Some(142.0));
        assert_eq!(activity.points[0].cadence, Some(79.0));
        assert_eq!(activity.points[1].heart_rate, Some(129.0));
        assert_eq!(activity.points[1].power, None);
    }

    #[test]
    fn rejects_invalid_xml() {
        assert!(matches!(parse("<nope"), Err(AppError::Invalid(_))));
    }

    fn track(times: &[(&str, bool)]) -> String {
        let points: String = times
            .iter()
            .map(|(time, gps)| {
                let position = if *gps {
                    "<Position><LatitudeDegrees>48.6</LatitudeDegrees><LongitudeDegrees>-3.8</LongitudeDegrees></Position><AltitudeMeters>70</AltitudeMeters>"
                } else {
                    ""
                };
                format!("<Trackpoint><Time>{time}</Time>{position}</Trackpoint>")
            })
            .collect();
        format!(
            r#"<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities><Activity Sport="Running"><Lap><Track>{points}</Track></Lap></Activity></Activities>
</TrainingCenterDatabase>"#
        )
    }

    fn times(xml: &str) -> Vec<String> {
        parse(xml)
            .unwrap()
            .points
            .iter()
            .map(|p| p.time.to_rfc3339())
            .collect()
    }

    #[test]
    fn closes_a_clock_jump_after_points_without_gps() {
        let xml = track(&[
            ("2026-10-31T09:25:23Z", false),
            ("2026-10-31T09:27:17Z", false),
            ("2026-11-14T13:16:04Z", true),
            ("2026-11-14T13:16:09Z", true),
        ]);
        assert_eq!(
            times(&xml),
            ["2026-10-31T09:27:18+00:00", "2026-10-31T09:27:23+00:00"]
        );
        assert_eq!(parse(&xml).unwrap().clock_jumps, 1);
    }

    #[test]
    fn keeps_a_long_pause() {
        let xml = track(&[
            ("2025-05-06T09:00:00Z", true),
            ("2025-05-06T11:00:00Z", true),
        ]);
        assert_eq!(
            times(&xml),
            ["2025-05-06T09:00:00+00:00", "2025-05-06T11:00:00+00:00"]
        );
    }
}
