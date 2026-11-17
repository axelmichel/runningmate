//! Activity totals and averages, ported from `TcxFileImporter.compute_data` and
//! `processing/data_processing.py` (`calculate_pace`, `detect_pauses`, `calculate_steps`).

use chrono::{DateTime, Local, Timelike, Utc};

use super::track::{PAUSE_THRESHOLD, Track};
use crate::activity_type::ViewMode;

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityMetrics {
    pub start: DateTime<Utc>,
    /// km, 2 decimals.
    pub distance: f64,
    /// Seconds.
    pub duration: i64,
    /// km/h, 2 decimals.
    pub avg_speed: f64,
    pub calories: i64,
    /// Metres.
    pub elevation_gain: i64,
    pub avg_heart_rate: i64,
    pub avg_power: i64,
    /// min/km, `NaN` when no point had a plausible pace.
    pub avg_pace: f64,
    /// Grade-adjusted pace, min/km: `avg_pace` times the distance over the flat distance that
    /// costs as much ([`flat_ratio`]), as a split's is formed from its pace. `None` for rides and
    /// without a plausible pace.
    pub avg_gap: Option<f64>,
    /// 5th / 95th percentile of the per-point pace.
    pub fastest_pace: f64,
    pub slowest_pace: f64,
    pub pause_minutes: f64,
    /// Steps per minute (both feet) and total steps, for runs and walks.
    pub avg_steps: i64,
    pub total_steps: i64,
}

/// Python's `round()`: halves go to the even neighbour.
fn round_even(value: f64, decimals: i32) -> f64 {
    let factor = 10f64.powi(decimals);
    (value * factor).round_ties_even() / factor
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values
        .filter(|v| !v.is_nan())
        .fold((0.0, 0usize), |(s, c), v| (s + v, c + 1));
    (count > 0).then(|| sum / count as f64)
}

/// numpy/pandas `quantile(q, interpolation="nearest")` over the non-NaN values.
fn quantile_nearest(values: &[f64], q: f64) -> f64 {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| !v.is_nan()).collect();
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted.sort_by(f64::total_cmp);
    let index = (q * (sorted.len() - 1) as f64).round_ties_even() as usize;
    sorted[index]
}

/// Distance over the flat distance costing as much, over the steps with a grade-adjusted pace:
/// what turns a pace into the grade-adjusted pace. `None` for rides, which have none. (A mean of
/// the per-point grade-adjusted paces would come out slower than the pace on a hilly loop: a
/// pace 1.3 times faster uphill and 1.3 times slower downhill averages to slower.)
fn flat_ratio(track: &Track) -> Option<f64> {
    let (distance, flat) =
        (0..track.len())
            .filter(|i| track.gap[*i].is_finite())
            .fold((0.0, 0.0), |(d, f), i| {
                (
                    d + track.geodesic[i],
                    f + track.geodesic[i] * track.grade_factor[i],
                )
            });
    (flat > 0.0).then(|| distance / flat)
}

pub fn compute(track: &Track) -> ActivityMetrics {
    let points = &track.points;
    let start = points.first().map_or(DateTime::UNIX_EPOCH, |p| p.time);
    let end = points.last().map_or(start, |p| p.time);
    let duration = (end - start).num_seconds();
    let total_distance = track.distance_km.last().copied().unwrap_or(0.0);
    let avg_speed = if duration > 0 {
        total_distance / (duration as f64 / 3600.0)
    } else {
        0.0
    };

    let elevation_gain: f64 = points
        .windows(2)
        .map(|w| (w[1].elevation - w[0].elevation).max(0.0))
        .sum();

    // Pauses use the raw gaps. Python used the smoothed ones, which spread a pause in 1 s
    // recordings over 5 points so it rarely crossed the threshold.
    let pause_seconds: f64 = track
        .raw_time_diff
        .iter()
        .filter(|d| **d > PAUSE_THRESHOLD)
        .sum();

    let (avg_steps, total_steps) = if matches!(track.view, ViewMode::Running | ViewMode::Walking) {
        // TCX cadence counts one foot.
        let steps: Vec<f64> = points
            .iter()
            .map(|p| p.cadence.map_or(f64::NAN, |c| c * 2.0))
            .collect();
        // A pause isn't walked through at the cadence of the point after it.
        let total: f64 = steps
            .iter()
            .zip(&track.raw_time_diff)
            .filter(|(_, t)| **t <= PAUSE_THRESHOLD)
            .map(|(s, t)| s * t / 60.0)
            .filter(|v| !v.is_nan())
            .sum();
        (
            mean(steps.iter().copied()).map_or(0, |a| round_even(a, 0) as i64),
            total as i64,
        )
    } else {
        (0, 0)
    };

    ActivityMetrics {
        start,
        distance: round_even(total_distance, 2),
        duration,
        avg_speed: round_even(avg_speed, 2),
        calories: round_even(track.calories.iter().sum(), 0) as i64,
        elevation_gain: round_even(elevation_gain, 0) as i64,
        avg_heart_rate: mean(points.iter().map(|p| p.heart_rate.unwrap_or(f64::NAN)))
            .map_or(0, |hr| round_even(hr, 0) as i64),
        avg_power: mean(track.power.iter().copied()).map_or(0, |p| round_even(p, 0) as i64),
        avg_pace: mean(track.pace.iter().copied()).unwrap_or(f64::NAN),
        avg_gap: flat_ratio(track)
            .zip(mean(track.pace.iter().copied()))
            .map(|(ratio, pace)| pace * ratio),
        fastest_pace: quantile_nearest(&track.pace, 0.05),
        slowest_pace: quantile_nearest(&track.pace, 0.95),
        pause_minutes: pause_seconds / 60.0,
        avg_steps,
        total_steps,
    }
}

/// Minutes → "MM:SS", as stored in the per-type tables. Seconds are rounded first, so the
/// Python app's "08:60" can't occur.
pub fn format_min_sec(minutes: f64) -> String {
    if !minutes.is_finite() || minutes < 0.0 {
        return "00:00".into();
    }
    let total = (minutes * 60.0).round() as i64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// "Run in the Afternoon" and friends, from `generate_activity_title`, in the app language.
/// Uses the local hour of the start (Python used UTC).
pub fn activity_title(view: ViewMode, start: DateTime<Utc>, language: &str) -> String {
    let hour = start.with_timezone(&Local).hour();
    let german = language == "de";
    let time_of_day = match (hour, german) {
        (5..=11, false) => "in the Morning",
        (12..=16, false) => "in the Afternoon",
        (17..=20, false) => "in the Evening",
        (_, false) => "at Night",
        (5..=11, true) => "am Morgen",
        (12..=16, true) => "am Nachmittag",
        (17..=20, true) => "am Abend",
        (_, true) => "in der Nacht",
    };
    let label = match (view, german) {
        (ViewMode::Running, false) => "Run",
        (ViewMode::Cycling, false) => "Ride",
        (ViewMode::Walking, false) => "Walk",
        (ViewMode::Running, true) => "Lauf",
        (ViewMode::Cycling, true) => "Radfahrt",
        (ViewMode::Walking, true) => "Spaziergang",
        (ViewMode::All, _) => "Activity",
    };
    format!("{label} {time_of_day}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_matches_numpy_nearest() {
        let values = [5.0, f64::NAN, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(quantile_nearest(&values, 0.05), 1.0);
        assert_eq!(quantile_nearest(&values, 0.95), 5.0);
        // 0.5 * 4 = 2 → third value; ties round to even like numpy.
        assert_eq!(quantile_nearest(&[1.0, 2.0, 3.0, 4.0], 0.5), 3.0);
    }

    #[test]
    fn steps_are_not_counted_through_a_pause() {
        let point = |second: i64| super::super::tcx::RawPoint {
            time: DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(second),
            latitude: 48.0 + second as f64 * 1e-5,
            longitude: -3.8,
            elevation: 50.0,
            heart_rate: None,
            cadence: Some(80.0),
            power: None,
        };
        // 60 s at 160 steps per minute, then a 600 s pause, then 60 s more.
        let points = (0..=60).chain(660..=720).map(point).collect();
        let metrics = compute(&Track::new(points, ViewMode::Running, 70.0));
        assert_eq!(metrics.total_steps, 2 * 160);
        assert_eq!(metrics.pause_minutes, 10.0);
    }

    #[test]
    fn formats_minutes_without_sixty_seconds() {
        assert_eq!(format_min_sec(6.742), "06:45");
        assert_eq!(format_min_sec(8.9999), "09:00");
        assert_eq!(format_min_sec(f64::NAN), "00:00");
    }

    #[test]
    fn titles_follow_language() {
        let start = Local::now().with_hour(14).unwrap().with_timezone(&Utc);
        assert_eq!(
            activity_title(ViewMode::Running, start, "en"),
            "Run in the Afternoon"
        );
        assert_eq!(
            activity_title(ViewMode::Cycling, start, "de"),
            "Radfahrt am Nachmittag"
        );
    }
}
