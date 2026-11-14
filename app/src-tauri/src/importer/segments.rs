//! Distance segments (`activity_details` rows) and best segments, ported from
//! `processing/tcx_segment_parser.py` and `BestSegmentFinder._get_best_cumulative_segment`.

use chrono::{DateTime, Utc};

use super::track::Track;
use crate::activity_type::ViewMode;

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub latitude: f64,
    pub longitude: f64,
    pub avg_heart_rate: i64,
    pub avg_power: i64,
    /// m/s; `None` if no point had a speed.
    pub avg_speed: Option<f64>,
    /// min/km over the segment's elapsed time.
    pub avg_pace: f64,
    /// Steps per minute of one foot; `None` for rides.
    pub avg_steps: Option<i64>,
    /// km.
    pub distance: f64,
    /// Metres.
    pub elevation_gain: f64,
    pub time_start: DateTime<Utc>,
    pub time_end: DateTime<Utc>,
}

/// Segment length in km: 5 for rides, 1 otherwise.
pub fn segment_length(view: ViewMode) -> f64 {
    if view == ViewMode::Cycling { 5.0 } else { 1.0 }
}

/// pandas `str(Timestamp)` for UTC times, the format stored in `seg_time_start`/`seg_time_end`.
pub fn format_time(time: DateTime<Utc>) -> String {
    if time.timestamp_subsec_nanos() == 0 {
        time.format("%Y-%m-%d %H:%M:%S+00:00").to_string()
    } else {
        time.format("%Y-%m-%d %H:%M:%S%.6f+00:00").to_string()
    }
}

/// Values collected for the segment being built.
#[derive(Default)]
struct Accumulator {
    heart_rate: Vec<f64>,
    power: Vec<f64>,
    speed: Vec<f64>,
    steps: Vec<f64>,
    start: Option<(f64, f64, DateTime<Utc>)>,
    distance: f64,
    elevation_gain: f64,
}

fn average(values: &[f64]) -> Option<f64> {
    let valid: Vec<f64> = values.iter().copied().filter(|v| !v.is_nan()).collect();
    (!valid.is_empty()).then(|| valid.iter().sum::<f64>() / valid.len() as f64)
}

/// Python `safe_round`: `None`/NaN → 0, otherwise round half to even.
fn safe_round(value: Option<f64>) -> i64 {
    value
        .filter(|v| v.is_finite())
        .map_or(0, |v| v.round_ties_even() as i64)
}

impl Accumulator {
    fn finish(self, end: DateTime<Utc>, view: ViewMode) -> Option<Segment> {
        let (latitude, longitude, start) = self.start?;
        let minutes = (end - start).as_seconds_f64() / 60.0;
        let avg_pace = if minutes != 0.0 && self.distance != 0.0 {
            minutes / self.distance
        } else {
            0.0
        };
        let steps = average(&self.steps).filter(|s| *s != 0.0);
        Some(Segment {
            latitude,
            longitude,
            avg_heart_rate: safe_round(average(&self.heart_rate)),
            avg_power: safe_round(average(&self.power)),
            avg_speed: average(&self.speed),
            avg_pace,
            avg_steps: (view != ViewMode::Cycling).then(|| safe_round(steps)),
            distance: self.distance,
            elevation_gain: self.elevation_gain,
            time_start: start,
            time_end: end,
        })
    }
}

/// Splits the track into segments of `segment_length` km.
///
/// The cut is decided on the segment distance (the sum of the raw geodesic steps), so a full
/// segment is never shorter than its length. (Python cut on the step distance with standing
/// still interpolated, which a stop at a traffic light could inflate by 100 m or more, so its
/// "1 km" segments were often shorter.) Each segment starts where the previous one ended, so
/// the step into it counts with its time as well as its distance.
pub fn split(track: &Track) -> Vec<Segment> {
    let length = segment_length(track.view);
    let mut segments = Vec::new();
    let mut current = Accumulator::default();
    let mut last_time = None;

    for i in 0..track.len() {
        let point = &track.points[i];
        last_time = Some(point.time);
        if current.start.is_none() {
            current.start = Some((point.latitude, point.longitude, point.time));
        }
        current
            .heart_rate
            .push(point.heart_rate.unwrap_or(f64::NAN));
        current.power.push(track.power[i]);
        current.steps.push(point.cadence.unwrap_or(f64::NAN));
        current.speed.push(track.speed[i]);
        if i > 0 {
            let rise = point.elevation - track.points[i - 1].elevation;
            if rise > 0.0 {
                current.elevation_gain += rise;
            }
        }
        current.distance += track.geodesic[i] / 1000.0;
        if current.distance >= length {
            segments.extend(std::mem::take(&mut current).finish(point.time, track.view));
            current.start = Some((point.latitude, point.longitude, point.time));
        }
    }
    if current.distance > 0.0
        && let Some(end) = last_time
    {
        segments.extend(current.finish(end, track.view));
    }
    segments
}

#[derive(Debug, Clone, PartialEq)]
pub struct BestSegment {
    /// Target distance in km.
    pub distance: f64,
    /// Time over distance of the segments covering it, min/km.
    pub pace: f64,
    pub time_start: DateTime<Utc>,
}

/// Fastest run of consecutive segments covering each target distance, its pace being the
/// segments' time over their distance. (Python averaged the segment paces, so a short last
/// segment weighed as much as a full one, and one with no time counted as pace 0.)
pub fn best_segments(segments: &[Segment], targets: &[f64]) -> Vec<BestSegment> {
    targets
        .iter()
        .filter_map(|&target| {
            let mut best: Option<BestSegment> = None;
            for start in 0..segments.len() {
                let mut distance = 0.0;
                let mut seconds = 0.0;
                for segment in &segments[start..] {
                    distance += segment.distance;
                    seconds += (segment.time_end - segment.time_start).as_seconds_f64();
                    if distance < target {
                        continue;
                    }
                    let pace = seconds / 60.0 / distance;
                    if best.as_ref().is_none_or(|b| pace < b.pace) {
                        best = Some(BestSegment {
                            distance: target,
                            pace,
                            time_start: segments[start].time_start,
                        });
                    }
                    break;
                }
            }
            best
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A segment starting `minute` minutes in and taking `pace × distance` minutes.
    fn segment(distance: f64, pace: f64, minute: i64) -> Segment {
        let start = DateTime::UNIX_EPOCH + chrono::Duration::minutes(minute);
        let end = start + chrono::Duration::milliseconds((pace * distance * 60_000.0) as i64);
        Segment {
            latitude: 0.0,
            longitude: 0.0,
            avg_heart_rate: 0,
            avg_power: 0,
            avg_speed: None,
            avg_pace: pace,
            avg_steps: None,
            distance,
            elevation_gain: 0.0,
            time_start: start,
            time_end: end,
        }
    }

    #[test]
    fn finds_fastest_consecutive_segments() {
        let segments = [
            segment(1.0, 6.0, 0),
            segment(1.0, 5.0, 6),
            segment(1.0, 5.5, 11),
            segment(0.5, 4.0, 17),
        ];
        let best = best_segments(&segments, &[1.0, 2.0, 5.0]);
        assert_eq!(best.len(), 2);
        assert_eq!((best[0].distance, best[0].pace), (1.0, 5.0));
        assert_eq!(best[0].time_start, segments[1].time_start);
        // 1 km + 1 km from the second segment: (5.0 + 5.5) / 2.
        assert_eq!((best[1].distance, best[1].pace), (2.0, 5.25));
    }

    #[test]
    fn weighs_segments_by_distance() {
        // 1 km at 6:00 and a 0.2 km remainder at 4:00 make 1.2 km in 6.8 minutes, not the
        // mean of the two paces (5:00).
        let segments = [segment(1.0, 6.0, 0), segment(0.2, 4.0, 6)];
        let best = best_segments(&segments, &[1.2]);
        assert!((best[0].pace - 6.8 / 1.2).abs() < 1e-9, "{}", best[0].pace);
        // A remainder without time doesn't halve the pace (Python: (6.0 + 0.0) / 2).
        let segments = [segment(0.998, 6.0, 0), segment(0.01, 0.0, 6)];
        let best = best_segments(&segments, &[1.0]);
        assert!(
            (best[0].pace - 0.998 * 6.0 / 1.008).abs() < 1e-6,
            "{}",
            best[0].pace
        );
    }

    #[test]
    fn formats_times_like_pandas() {
        let t = DateTime::parse_from_rfc3339("2025-05-06T11:18:34Z")
            .unwrap()
            .to_utc();
        assert_eq!(format_time(t), "2025-05-06 11:18:34+00:00");
        let t = DateTime::parse_from_rfc3339("2025-05-06T11:18:34.5Z")
            .unwrap()
            .to_utc();
        assert_eq!(format_time(t), "2025-05-06 11:18:34.500000+00:00");
    }
}
