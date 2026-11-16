//! TCX (Garmin Training Center XML) reading, the XML part of `processing/tcx_file_parser.py`.

use std::fmt::Write;

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use geographiclib_rs::{Geodesic, InverseGeodesic};
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

/// Any `<Trackpoint>` with a time, also without position or altitude (no GPS yet, a dropout or
/// an indoor run). The route fix editor fills in the missing positions.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSample {
    pub time: DateTime<Utc>,
    /// Latitude and longitude.
    pub position: Option<(f64, f64)>,
    pub elevation: Option<f64>,
    pub heart_rate: Option<f64>,
    pub cadence: Option<f64>,
    pub power: Option<f64>,
    /// `DistanceMeters` as measured by the device (also from a foot pod or wheel sensor).
    pub distance: Option<f64>,
}

impl RawSample {
    /// The sample as a [`RawPoint`] when it has a position and an altitude.
    pub fn point(&self) -> Option<RawPoint> {
        let (latitude, longitude) = self.position?;
        Some(RawPoint {
            time: self.time,
            latitude,
            longitude,
            elevation: self.elevation?,
            heart_rate: self.heart_rate,
            cadence: self.cadence,
            power: self.power,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TcxActivity {
    /// `Sport` attribute of the first `<Activity>` ("Running", "Biking", "Other", …).
    pub sport: String,
    /// The samples with position and altitude, which everything is computed from.
    pub points: Vec<RawPoint>,
    /// All samples with a time, in order.
    pub samples: Vec<RawSample>,
    /// Clock jumps closed by [`without_clock_jumps`].
    pub clock_jumps: usize,
}

impl TcxActivity {
    /// Builds the activity from timed samples: closes clock jumps across all of them, so that
    /// samples without a position still anchor the start, then picks the positioned points.
    pub fn from_samples(sport: String, mut samples: Vec<RawSample>) -> Self {
        let (times, clock_jumps) = without_clock_jumps(samples.iter().map(|s| s.time).collect());
        for (sample, time) in samples.iter_mut().zip(times) {
            sample.time = time;
        }
        Self {
            sport,
            points: samples.iter().filter_map(RawSample::point).collect(),
            samples,
            clock_jumps,
        }
    }
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
pub(crate) fn without_clock_jumps(mut times: Vec<DateTime<Utc>>) -> (Vec<DateTime<Utc>>, usize) {
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
/// left out of `points` (they stay in `samples`). Unlike it, heart rate stays with its own
/// trackpoint: Python collected heart rates into a separate list and lined them up by index,
/// which shifted them whenever some points had no heart rate. Clock jumps are removed first,
/// across all trackpoints, so that points without a position still anchor the start (see
/// [`without_clock_jumps`]).
pub fn parse(xml: &str) -> AppResult<TcxActivity> {
    let doc = Document::parse(xml).map_err(|e| AppError::Invalid(format!("invalid TCX: {e}")))?;
    let root = doc.root_element();

    let sport = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Activity")
        .and_then(|n| n.attribute("Sport"))
        .unwrap_or("Unknown")
        .to_string();

    let samples = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Trackpoint")
        .filter_map(|tp| {
            let time = child(tp, "Time")?.text()?.trim();
            let time = DateTime::parse_from_rfc3339(time).ok()?.with_timezone(&Utc);
            let extension = tp.descendants().find(|n| {
                n.is_element()
                    && n.tag_name().name() == "TPX"
                    && n.tag_name().namespace() == Some(ACTIVITY_EXTENSION_NS)
            });
            let position = number(tp, "LatitudeDegrees").zip(number(tp, "LongitudeDegrees"));
            Some(RawSample {
                time,
                position,
                elevation: number(tp, "AltitudeMeters"),
                heart_rate: child(tp, "HeartRateBpm").and_then(|hr| number(hr, "Value")),
                cadence: extension.and_then(|e| number(e, "RunCadence")),
                power: extension.and_then(|e| number(e, "Watts")),
                // The lap has a `DistanceMeters` too; only the trackpoint's own one counts.
                distance: tp
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "DistanceMeters")
                    .and_then(|n| n.text()?.trim().parse().ok()),
            })
        })
        .collect();

    Ok(TcxActivity::from_samples(sport, samples))
}

fn time(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

/// Writes `activity` as a TCX document with one lap holding all samples; [`parse`] reads it
/// back unchanged. Used for GPX imports and repaired tracks, which are archived as TCX.
pub fn write(activity: &TcxActivity) -> String {
    let samples = &activity.samples;
    let (Some(first), Some(last)) = (samples.first(), samples.last()) else {
        return String::new();
    };
    let geodesic = Geodesic::wgs84();
    let distance: f64 = activity
        .points
        .windows(2)
        .map(|w| -> f64 {
            geodesic.inverse(w[0].latitude, w[0].longitude, w[1].latitude, w[1].longitude)
        })
        .sum();
    let seconds = (last.time - first.time).num_milliseconds() as f64 / 1000.0;
    let start = time(first.time);

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(
        "<TrainingCenterDatabase xmlns=\"http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2\">\n",
    );
    let sport = escape(&activity.sport);
    let _ = writeln!(
        xml,
        "<Activities><Activity Sport=\"{sport}\"><Id>{start}</Id>"
    );
    let _ = writeln!(
        xml,
        "<Lap StartTime=\"{start}\"><TotalTimeSeconds>{seconds}</TotalTimeSeconds><DistanceMeters>{distance:.1}</DistanceMeters><Calories>0</Calories><Intensity>Active</Intensity><TriggerMethod>Manual</TriggerMethod><Track>"
    );
    for sample in samples {
        let _ = write!(xml, "<Trackpoint><Time>{}</Time>", time(sample.time));
        if let Some((lat, lon)) = sample.position {
            let _ = write!(
                xml,
                "<Position><LatitudeDegrees>{lat}</LatitudeDegrees><LongitudeDegrees>{lon}</LongitudeDegrees></Position>"
            );
        }
        if let Some(ele) = sample.elevation {
            let _ = write!(xml, "<AltitudeMeters>{ele}</AltitudeMeters>");
        }
        if let Some(distance) = sample.distance {
            let _ = write!(xml, "<DistanceMeters>{distance}</DistanceMeters>");
        }
        if let Some(hr) = sample.heart_rate {
            let _ = write!(xml, "<HeartRateBpm><Value>{hr}</Value></HeartRateBpm>");
        }
        if sample.cadence.is_some() || sample.power.is_some() {
            let _ = write!(xml, "<Extensions><TPX xmlns=\"{ACTIVITY_EXTENSION_NS}\">");
            if let Some(watts) = sample.power {
                let _ = write!(xml, "<Watts>{watts}</Watts>");
            }
            if let Some(cadence) = sample.cadence {
                let _ = write!(xml, "<RunCadence>{cadence}</RunCadence>");
            }
            xml.push_str("</TPX></Extensions>");
        }
        xml.push_str("</Trackpoint>\n");
    }
    xml.push_str("</Track></Lap></Activity></Activities>\n</TrainingCenterDatabase>\n");
    xml
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
    fn keeps_samples_without_position() {
        let activity = parse(SAMPLE).unwrap();
        assert_eq!(activity.samples.len(), 3);
        assert_eq!(activity.samples[1].position, None);
        assert_eq!(activity.samples[1].elevation, Some(72.0));
        assert_eq!(activity.samples[1].heart_rate, Some(120.0));
    }

    #[test]
    fn reads_trackpoint_distance_not_lap_distance() {
        let xml = r#"<TrainingCenterDatabase><Activities><Activity Sport="Running">
  <Lap><DistanceMeters>5000</DistanceMeters><Track>
    <Trackpoint><Time>2025-05-06T11:18:34Z</Time><DistanceMeters>12.5</DistanceMeters></Trackpoint>
    <Trackpoint><Time>2025-05-06T11:18:35Z</Time></Trackpoint>
  </Track></Lap></Activity></Activities></TrainingCenterDatabase>"#;
        let activity = parse(xml).unwrap();
        assert_eq!(activity.samples[0].distance, Some(12.5));
        assert_eq!(activity.samples[1].distance, None);
        assert!(activity.points.is_empty());
    }

    #[test]
    fn written_tcx_reads_back_the_same() {
        let activity = parse(SAMPLE).unwrap();
        let again = parse(&write(&activity)).unwrap();
        assert_eq!(again.sport, activity.sport);
        assert_eq!(again.samples, activity.samples);
        assert_eq!(again.points, activity.points);
    }

    #[test]
    fn written_fixture_reads_back_the_same() {
        let xml = include_str!("../../tests/fixtures/synthetic_run.tcx");
        let activity = parse(xml).unwrap();
        assert_eq!(parse(&write(&activity)).unwrap().samples, activity.samples);
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
