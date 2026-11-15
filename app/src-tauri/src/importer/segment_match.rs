//! Route segments: cutting a stretch out of a track, and finding every pass over it in other
//! tracks. Distances are measured in a local flat projection around the segment, which is
//! accurate to well under a metre over the few kilometres a segment spans.

use serde::{Deserialize, Serialize};

use super::track::{Track, pace_limits};

/// A pass must come this close (m) to the segment's start and end.
const ENDPOINT_RADIUS: f64 = 35.0;
/// Every sample of the segment must lie this close (m) to the pass.
const ROUTE_TOLERANCE: f64 = 50.0;
/// Spacing (m) of the segment samples checked against a pass.
const SAMPLE_SPACING: f64 = 25.0;
/// Distance travelled on a pass, as a share of the segment's length.
const MIN_LENGTH_RATIO: f64 = 0.8;
const MAX_LENGTH_RATIO: f64 = 1.25;
/// Window (m) over which the steepest grade is measured, so GPS noise doesn't make spikes.
const GRADE_WINDOW: f64 = 100.0;
/// Points kept for the map and the profile: about one every few metres, at most this many.
const MAX_SHAPE_POINTS: usize = 400;
const MIN_POINT_SPACING: f64 = 5.0;
/// Shortest segment (m) that can be stored.
pub const MIN_SEGMENT_LENGTH: f64 = 100.0;

const EARTH_RADIUS: f64 = 6_371_008.8;

/// One point of a segment: latitude, longitude, elevation (m), distance from its start (m).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShapePoint(pub f64, pub f64, pub f64, pub f64);

impl ShapePoint {
    pub fn latitude(&self) -> f64 {
        self.0
    }
    pub fn longitude(&self) -> f64 {
        self.1
    }
    pub fn elevation(&self) -> f64 {
        self.2
    }
    pub fn distance(&self) -> f64 {
        self.3
    }
}

/// A stretch cut out of a track, with its numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentShape {
    pub points: Vec<ShapePoint>,
    /// m.
    pub distance: f64,
    /// m.
    pub elevation_gain: f64,
    pub elevation_loss: f64,
    /// %, from the start to the end elevation.
    pub avg_grade: f64,
    /// %, steepest climb over [`GRADE_WINDOW`].
    pub max_grade: f64,
    pub min_elevation: f64,
    pub max_elevation: f64,
}

/// One pass over a segment.
#[derive(Debug, Clone, PartialEq)]
pub struct Effort {
    /// Position within the activity, km.
    pub start_km: f64,
    pub end_km: f64,
    /// s.
    pub elapsed: f64,
    pub avg_heart_rate: Option<f64>,
    /// Unix timestamp of the start of the pass.
    pub date: i64,
}

/// Flat projection (m) around an origin.
#[derive(Debug, Clone, Copy)]
struct Projection {
    latitude: f64,
    longitude: f64,
    cos_latitude: f64,
}

impl Projection {
    fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            cos_latitude: latitude.to_radians().cos(),
        }
    }

    fn project(&self, latitude: f64, longitude: f64) -> (f64, f64) {
        (
            (longitude - self.longitude).to_radians() * self.cos_latitude * EARTH_RADIUS,
            (latitude - self.latitude).to_radians() * EARTH_RADIUS,
        )
    }
}

type Xy = (f64, f64);

fn distance(a: Xy, b: Xy) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Distance from `p` to the line from `a` to `b`, and how far along it (0–1) the closest point is.
fn to_edge(p: Xy, a: Xy, b: Xy) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    };
    (distance(p, (a.0 + t * dx, a.1 + t * dy)), t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Value of a per-point series at a fractional index.
fn at(values: impl Fn(usize) -> f64, position: f64, n: usize) -> f64 {
    let i = (position.floor() as usize).min(n - 1);
    if i + 1 >= n {
        return values(i);
    }
    lerp(values(i), values(i + 1), position - i as f64)
}

/// Fractional index where the cumulative distance reaches `km`.
fn position_at_km(distance_km: &[f64], km: f64) -> f64 {
    let n = distance_km.len();
    let i = distance_km.partition_point(|d| *d < km);
    if i == 0 {
        return 0.0;
    }
    if i >= n {
        return (n - 1) as f64;
    }
    let (a, b) = (distance_km[i - 1], distance_km[i]);
    let t = if b > a { (km - a) / (b - a) } else { 0.0 };
    (i - 1) as f64 + t
}

/// The stretch of `track` between `start_km` and `end_km` (cumulative distance), or `None` if
/// it is shorter than [`MIN_SEGMENT_LENGTH`].
pub fn cut(track: &Track, start_km: f64, end_km: f64) -> Option<SegmentShape> {
    let n = track.len();
    if n < 2 {
        return None;
    }
    let (start_km, end_km) = (start_km.min(end_km), start_km.max(end_km));
    let from = position_at_km(&track.distance_km, start_km);
    let to = position_at_km(&track.distance_km, end_km);
    let point_at = |position: f64| {
        let p = |f: fn(&super::tcx::RawPoint) -> f64| at(|i| f(&track.points[i]), position, n);
        (p(|p| p.latitude), p(|p| p.longitude), p(|p| p.elevation))
    };
    let mut raw = vec![point_at(from)];
    for i in (from.floor() as usize + 1)..=(to.ceil() as usize).min(n - 1) {
        if (i as f64) < to {
            let p = &track.points[i];
            raw.push((p.latitude, p.longitude, p.elevation));
        }
    }
    raw.push(point_at(to));

    let origin = Projection::new(raw[0].0, raw[0].1);
    let mut points = Vec::with_capacity(raw.len());
    let mut travelled = 0.0;
    let mut previous = origin.project(raw[0].0, raw[0].1);
    for (latitude, longitude, elevation) in &raw {
        let xy = origin.project(*latitude, *longitude);
        travelled += distance(previous, xy);
        previous = xy;
        points.push(ShapePoint(*latitude, *longitude, *elevation, travelled));
    }
    if travelled < MIN_SEGMENT_LENGTH {
        return None;
    }
    let (elevation_gain, elevation_loss) = points.windows(2).fold((0.0, 0.0), |(up, down), w| {
        let rise = w[1].elevation() - w[0].elevation();
        (up + rise.max(0.0), down + (-rise).max(0.0))
    });
    let first = points[0];
    let last = points[points.len() - 1];
    Some(SegmentShape {
        distance: travelled,
        elevation_gain,
        elevation_loss,
        avg_grade: (last.elevation() - first.elevation()) / travelled * 100.0,
        max_grade: max_grade(&points),
        min_elevation: points
            .iter()
            .map(|p| p.elevation())
            .fold(f64::MAX, f64::min),
        max_elevation: points
            .iter()
            .map(|p| p.elevation())
            .fold(f64::MIN, f64::max),
        points: thin(&points),
    })
}

/// Steepest grade (%) between two points at least [`GRADE_WINDOW`] apart (or the whole segment
/// if it is shorter).
fn max_grade(points: &[ShapePoint]) -> f64 {
    let total = points[points.len() - 1].distance();
    let window = GRADE_WINDOW.min(total);
    let mut steepest = f64::MIN;
    let mut j = 0;
    for p in points {
        while j < points.len() && points[j].distance() - p.distance() < window {
            j += 1;
        }
        if j == points.len() {
            break;
        }
        let q = points[j];
        steepest = steepest.max((q.elevation() - p.elevation()) / (q.distance() - p.distance()));
    }
    if steepest == f64::MIN {
        0.0
    } else {
        steepest * 100.0
    }
}

/// Keeps the first and last point and the others at least a few metres apart, at most
/// [`MAX_SHAPE_POINTS`].
fn thin(points: &[ShapePoint]) -> Vec<ShapePoint> {
    let total = points[points.len() - 1].distance();
    let spacing = (total / MAX_SHAPE_POINTS as f64).max(MIN_POINT_SPACING);
    let mut kept = vec![points[0]];
    for p in &points[1..points.len() - 1] {
        if p.distance() - kept[kept.len() - 1].distance() >= spacing
            && total - p.distance() >= spacing / 2.0
        {
            kept.push(*p);
        }
    }
    kept.push(points[points.len() - 1]);
    kept
}

/// Points along the segment every [`SAMPLE_SPACING`], including both ends.
fn samples(points: &[Xy], cumulative: &[f64]) -> Vec<Xy> {
    let total = cumulative[cumulative.len() - 1];
    let count = (total / SAMPLE_SPACING).ceil().max(1.0) as usize;
    let mut out = Vec::with_capacity(count + 1);
    let mut j = 0;
    for k in 0..=count {
        let d = total * k as f64 / count as f64;
        while j + 2 < cumulative.len() && cumulative[j + 1] < d {
            j += 1;
        }
        let span = cumulative[j + 1] - cumulative[j];
        let t = if span > 0.0 {
            ((d - cumulative[j]) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        out.push((
            lerp(points[j].0, points[j + 1].0, t),
            lerp(points[j].1, points[j + 1].1, t),
        ));
    }
    out
}

/// Closest approach of the track edges `from..to` to `p` within one cluster of edges closer than
/// `radius`, starting the search at edge `from`: `(edge, t, distance)`.
fn closest_cluster(
    track: &[Xy],
    p: Xy,
    radius: f64,
    from: usize,
    keep_going: impl Fn(usize) -> bool,
) -> Option<(usize, f64)> {
    let mut best: Option<(usize, f64, f64)> = None;
    for i in from..track.len().saturating_sub(1) {
        if !keep_going(i) {
            break;
        }
        let (d, t) = to_edge(p, track[i], track[i + 1]);
        if d <= radius {
            if best.is_none_or(|(_, _, bd)| d < bd) {
                best = Some((i, t, d));
            }
        } else if best.is_some() {
            break;
        }
    }
    best.map(|(i, t, _)| (i, t))
}

/// Every pass over `shape` in `track`, in order; laps count once each.
pub fn find_efforts(shape: &SegmentShape, track: &Track) -> Vec<Effort> {
    let n = track.len();
    if n < 2 || shape.points.len() < 2 {
        return Vec::new();
    }
    let first = shape.points[0];
    let origin = Projection::new(first.latitude(), first.longitude());
    let segment: Vec<Xy> = shape
        .points
        .iter()
        .map(|p| origin.project(p.latitude(), p.longitude()))
        .collect();
    let cumulative: Vec<f64> = shape.points.iter().map(|p| p.distance()).collect();
    let checks = samples(&segment, &cumulative);
    let (start, end) = (segment[0], segment[segment.len() - 1]);
    let length_km = shape.distance / 1000.0;

    let xy: Vec<Xy> = track
        .points
        .iter()
        .map(|p| origin.project(p.latitude, p.longitude))
        .collect();
    let km = |position: f64| at(|i| track.distance_km[i], position, n);
    let seconds = |position: f64| {
        let t0 = track.points[0].time;
        at(
            |i| (track.points[i].time - t0).as_seconds_f64(),
            position,
            n,
        )
    };
    let (fastest_pace, _, _) = pace_limits(track.view);

    let mut efforts = Vec::new();
    let mut search_from = 0;
    while let Some((start_edge, start_t)) =
        closest_cluster(&xy, start, ENDPOINT_RADIUS, search_from, |_| true)
    {
        let start_position = start_edge as f64 + start_t;
        let start_km = km(start_position);
        // The end, once at least most of the segment's length has been covered.
        let min_km = start_km + length_km * MIN_LENGTH_RATIO;
        let max_km = start_km + length_km * MAX_LENGTH_RATIO;
        let first_end_edge = track.distance_km.partition_point(|d| *d < min_km).max(1) - 1;
        let found = closest_cluster(
            &xy,
            end,
            ENDPOINT_RADIUS,
            first_end_edge.max(start_edge),
            |i| track.distance_km[i] <= max_km,
        );
        // Look for the next pass after this cluster of start edges.
        search_from = next_cluster(&xy, start, start_edge);
        let Some((end_edge, end_t)) = found else {
            continue;
        };
        let end_position = end_edge as f64 + end_t;
        let end_km = km(end_position);
        let travelled = end_km - start_km;
        if !(length_km * MIN_LENGTH_RATIO..=length_km * MAX_LENGTH_RATIO).contains(&travelled) {
            continue;
        }
        let edges = start_edge..=end_edge.min(n - 2);
        let follows = checks.iter().all(|s| {
            edges
                .clone()
                .any(|i| to_edge(*s, xy[i], xy[i + 1]).0 <= ROUTE_TOLERANCE)
        });
        if !follows {
            continue;
        }
        // No pass across a clock running backwards.
        let clock_ok = (start_edge + 1..=end_edge.min(n - 2) + 1)
            .all(|i| track.raw_time_diff[i].is_nan() || track.raw_time_diff[i] >= 0.0);
        let elapsed = seconds(end_position) - seconds(start_position);
        if !clock_ok || elapsed <= 0.0 {
            continue;
        }
        let pace = elapsed / 60.0 / length_km;
        if pace < fastest_pace {
            continue;
        }
        let heart_rates: Vec<f64> = (start_edge..=end_edge.min(n - 1) + 1)
            .filter_map(|i| track.points.get(i).and_then(|p| p.heart_rate))
            .collect();
        let start_time = track.points[0].time.timestamp() as f64 + seconds(start_position);
        efforts.push(Effort {
            start_km,
            end_km,
            elapsed,
            avg_heart_rate: (!heart_rates.is_empty())
                .then(|| heart_rates.iter().sum::<f64>() / heart_rates.len() as f64),
            date: start_time.round() as i64,
        });
        search_from = search_from.max(end_edge + 1);
    }
    efforts
}

/// First edge after the cluster of edges near `p` that starts at `edge`.
fn next_cluster(track: &[Xy], p: Xy, edge: usize) -> usize {
    let mut i = edge;
    while i + 1 < track.len() && to_edge(p, track[i], track[i + 1]).0 <= ENDPOINT_RADIUS {
        i += 1;
    }
    i + 1
}

/// Latitude/longitude box around the segment, widened by `margin_km`: `(south, west, north, east)`.
pub fn bounding_box(points: &[ShapePoint], margin_km: f64) -> (f64, f64, f64, f64) {
    let (mut south, mut west, mut north, mut east) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in points {
        south = south.min(p.latitude());
        north = north.max(p.latitude());
        west = west.min(p.longitude());
        east = east.max(p.longitude());
    }
    let lat_margin = (margin_km * 1000.0 / EARTH_RADIUS).to_degrees();
    let cos = ((south + north) / 2.0).to_radians().cos().max(0.01);
    let lon_margin = lat_margin / cos;
    (
        south - lat_margin,
        west - lon_margin,
        north + lat_margin,
        east + lon_margin,
    )
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, Utc};

    use super::*;
    use crate::{activity_type::ViewMode, importer::tcx::RawPoint};

    const METRES_PER_DEGREE: f64 = EARTH_RADIUS * std::f64::consts::PI / 180.0;
    const LATITUDE: f64 = 48.0;

    fn start_time() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    /// Point `north` and `east` metres from the origin.
    fn at_metres(north: f64, east: f64) -> (f64, f64) {
        (
            LATITUDE + north / METRES_PER_DEGREE,
            11.0 + east / (METRES_PER_DEGREE * LATITUDE.to_radians().cos()),
        )
    }

    /// A track through `(north, east, elevation)` waypoints, a point every `step` metres at
    /// `speed` m/s.
    fn track(waypoints: &[(f64, f64, f64)], step: f64, speed: f64) -> Track {
        let mut points = Vec::new();
        let mut time = start_time();
        for w in waypoints.windows(2) {
            let (a, b) = (w[0], w[1]);
            let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let steps = (length / step).ceil().max(1.0) as usize;
            for k in 0..steps {
                let t = k as f64 / steps as f64;
                let (lat, lon) = at_metres(lerp(a.0, b.0, t), lerp(a.1, b.1, t));
                points.push(RawPoint {
                    time,
                    latitude: lat,
                    longitude: lon,
                    elevation: lerp(a.2, b.2, t),
                    heart_rate: Some(150.0),
                    cadence: None,
                    power: None,
                });
                time += TimeDelta::milliseconds((length / steps as f64 / speed * 1000.0) as i64);
            }
        }
        let last = waypoints[waypoints.len() - 1];
        let (lat, lon) = at_metres(last.0, last.1);
        points.push(RawPoint {
            time,
            latitude: lat,
            longitude: lon,
            elevation: last.2,
            heart_rate: Some(150.0),
            cadence: None,
            power: None,
        });
        Track::new(points, ViewMode::Running, 70.0)
    }

    /// 1 km due north climbing 50 m, then 1 km east on the flat.
    fn climb_track() -> Track {
        track(
            &[
                (0.0, 0.0, 100.0),
                (1000.0, 0.0, 150.0),
                (1000.0, 1000.0, 150.0),
            ],
            5.0,
            3.0,
        )
    }

    #[test]
    fn cut_measures_distance_and_grade() {
        let track = climb_track();
        let shape = cut(&track, 0.0, 1.0).unwrap();
        assert!((shape.distance - 1000.0).abs() < 15.0, "{}", shape.distance);
        assert!((shape.elevation_gain - 50.0).abs() < 1.5);
        assert!(shape.elevation_loss < 0.5);
        assert!((shape.avg_grade - 5.0).abs() < 0.2, "{}", shape.avg_grade);
        assert!((shape.max_grade - 5.0).abs() < 0.3, "{}", shape.max_grade);
        assert!(shape.points.len() <= MAX_SHAPE_POINTS + 1);
    }

    #[test]
    fn cut_rejects_tiny_stretches() {
        assert!(cut(&climb_track(), 0.5, 0.52).is_none());
    }

    #[test]
    fn source_track_matches_itself() {
        let track = climb_track();
        let shape = cut(&track, 0.2, 0.8).unwrap();
        let efforts = find_efforts(&shape, &track);
        assert_eq!(efforts.len(), 1);
        // 600 m at 3 m/s.
        assert!(
            (efforts[0].elapsed - 200.0).abs() < 5.0,
            "{}",
            efforts[0].elapsed
        );
        assert_eq!(efforts[0].avg_heart_rate, Some(150.0));
        assert!((efforts[0].start_km - 0.2).abs() < 0.02);
    }

    #[test]
    fn every_lap_counts() {
        let shape = cut(&climb_track(), 0.2, 0.8).unwrap();
        // Up the hill, back down along a road 300 m to the east, up again.
        let laps = track(
            &[
                (0.0, 0.0, 100.0),
                (1000.0, 0.0, 150.0),
                (1000.0, 300.0, 150.0),
                (0.0, 300.0, 100.0),
                (0.0, 0.0, 100.0),
                (1000.0, 0.0, 150.0),
            ],
            5.0,
            3.0,
        );
        assert_eq!(find_efforts(&shape, &laps).len(), 2);
    }

    #[test]
    fn reverse_direction_does_not_count() {
        let shape = cut(&climb_track(), 0.2, 0.8).unwrap();
        let downhill = track(&[(1000.0, 0.0, 150.0), (0.0, 0.0, 100.0)], 5.0, 3.0);
        assert!(find_efforts(&shape, &downhill).is_empty());
    }

    #[test]
    fn parallel_road_does_not_count() {
        let shape = cut(&climb_track(), 0.2, 0.8).unwrap();
        // Starts and ends on the segment but runs 100 m to the side in between.
        let parallel = track(
            &[
                (0.0, 0.0, 100.0),
                (200.0, 0.0, 110.0),
                (300.0, 100.0, 115.0),
                (700.0, 100.0, 135.0),
                (800.0, 0.0, 140.0),
                (1000.0, 0.0, 150.0),
            ],
            5.0,
            3.0,
        );
        assert!(find_efforts(&shape, &parallel).is_empty());
    }

    #[test]
    fn detour_is_too_long() {
        let shape = cut(&climb_track(), 0.2, 0.8).unwrap();
        // Follows the segment but with an out-and-back of 400 m in the middle.
        let detour = track(
            &[
                (0.0, 0.0, 100.0),
                (500.0, 0.0, 125.0),
                (500.0, 200.0, 125.0),
                (500.0, 0.0, 125.0),
                (1000.0, 0.0, 150.0),
            ],
            5.0,
            3.0,
        );
        assert!(find_efforts(&shape, &detour).is_empty());
    }

    #[test]
    fn sparse_points_still_match() {
        let shape = cut(&climb_track(), 0.2, 0.8).unwrap();
        let sparse = track(
            &[
                (0.0, 0.0, 100.0),
                (1000.0, 0.0, 150.0),
                (1000.0, 1000.0, 150.0),
            ],
            45.0,
            3.0,
        );
        assert_eq!(find_efforts(&shape, &sparse).len(), 1);
    }

    #[test]
    fn clock_running_backwards_is_left_out() {
        let shape = cut(&climb_track(), 0.2, 0.8).unwrap();
        let mut broken = climb_track();
        let middle = broken.len() / 4;
        let mut points = broken.points.clone();
        for p in &mut points[middle..] {
            p.time -= TimeDelta::seconds(3600);
        }
        broken = Track::new(points, ViewMode::Running, 70.0);
        assert!(find_efforts(&shape, &broken).is_empty());
    }

    #[test]
    fn bounding_box_is_widened() {
        let shape = cut(&climb_track(), 0.0, 1.0).unwrap();
        let (south, west, north, east) = bounding_box(&shape.points, 1.0);
        assert!(south < LATITUDE - 0.008 && north > LATITUDE + 0.017);
        assert!(west < 11.0 - 0.012 && east > 11.0 + 0.012);
    }
}
