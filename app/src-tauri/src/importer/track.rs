//! Per-trackpoint derived values, ported from `processing/tcx_file_parser.py` and
//! `processing/data_processing.py`. Missing values are `NaN`, as in the pandas original, so the
//! formulas read the same.

use geographiclib_rs::{Geodesic, InverseGeodesic};

use super::tcx::RawPoint;
use crate::activity_type::ViewMode;

const GRAVITY: f64 = 9.81;
/// Python's `16.6667`: s/m → min/km.
const PACE_FACTOR: f64 = 16.6667;
/// Default body and equipment parameters of `TcxFileParser`, used to estimate missing power.
const BODY_WEIGHT: f64 = 70.0;
const BIKE_WEIGHT: f64 = 8.0;
const ROLLING_RESISTANCE: f64 = 0.004;
const AIR_DENSITY: f64 = 1.225;
const DRAG_AREA: f64 = 0.27;
const RUN_SHOE_RESISTANCE: f64 = 0.02;
const WALK_SHOE_RESISTANCE: f64 = 0.03;

/// Pace limits (min/km) and the minimum step distance (m) per view, from `_map_pace`.
pub fn pace_limits(view: ViewMode) -> (f64, f64, f64) {
    match view {
        ViewMode::Running => (3.0, 12.0, 0.5),
        ViewMode::Cycling => (0.5, 6.0, 5.0),
        ViewMode::Walking => (8.0, 25.0, 0.2),
        ViewMode::All => (2.0, 20.0, 0.5),
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    pub view: ViewMode,
    pub points: Vec<RawPoint>,
    /// Seconds since the previous point (`NaN` for the first).
    pub raw_time_diff: Vec<f64>,
    /// Geodesic metres to the previous point (0 for the first).
    pub geodesic: Vec<f64>,
    /// `geodesic` with zeros interpolated, as pandas' `replace(0, NaN).interpolate()` does.
    pub dist_diff: Vec<f64>,
    /// m/s.
    pub speed: Vec<f64>,
    /// Measured watts, or estimated from speed when missing.
    pub power: Vec<f64>,
    /// min/km, `NaN` outside the plausible range for the activity type.
    pub pace: Vec<f64>,
    /// Cumulative km over UTM-projected coordinates.
    pub distance_km: Vec<f64>,
    pub calories: Vec<f64>,
}

impl Track {
    pub fn new(points: Vec<RawPoint>, view: ViewMode) -> Self {
        let n = points.len();
        let raw_time_diff: Vec<f64> = (0..n)
            .map(|i| {
                if i == 0 {
                    f64::NAN
                } else {
                    (points[i].time - points[i - 1].time).as_seconds_f64()
                }
            })
            .collect();

        let one_second = raw_time_diff.iter().filter(|d| **d == 1.0).count();
        // Smoothed over 5 points when more than 80 % of the gaps are exactly 1 s, as in Python.
        // Speed, pace and calories use this; pauses and steps use the raw gaps.
        let time_diff = if one_second as f64 > n as f64 * 0.8 {
            rolling_mean(&raw_time_diff, 5)
        } else {
            raw_time_diff.clone()
        };

        let geodesic_model = Geodesic::wgs84();
        let geodesic: Vec<f64> = (0..n)
            .map(|i| {
                if i == 0 {
                    0.0
                } else {
                    let (a, b) = (&points[i - 1], &points[i]);
                    geodesic_model.inverse(a.latitude, a.longitude, b.latitude, b.longitude)
                }
            })
            .collect();
        let dist_diff = interpolate(
            &geodesic
                .iter()
                .map(|d| if *d == 0.0 { f64::NAN } else { *d })
                .collect::<Vec<_>>(),
        );

        let speed: Vec<f64> = dist_diff
            .iter()
            .zip(&time_diff)
            .map(|(d, t)| if *t > 0.0 { d / t } else { f64::NAN })
            .collect();

        let power: Vec<f64> = points
            .iter()
            .zip(&speed)
            .map(|(p, v)| p.power.unwrap_or_else(|| estimate_power(view, *v)))
            .collect();

        let (min_pace, max_pace, min_dist) = pace_limits(view);
        let pace = dist_diff
            .iter()
            .zip(&time_diff)
            .map(|(d, t)| {
                if d.is_nan() || *d <= min_dist {
                    return f64::NAN;
                }
                let pace = t / d * PACE_FACTOR;
                if pace.is_finite() && (min_pace..=max_pace).contains(&pace) {
                    pace
                } else {
                    f64::NAN
                }
            })
            .collect();

        let distance_km = utm_cumulative_km(&points);

        let efficiency = if view == ViewMode::Cycling {
            0.25
        } else {
            0.20
        };
        let calories = power
            .iter()
            .zip(&time_diff)
            .map(|(p, t)| {
                if p.is_nan() || t.is_nan() {
                    0.0
                } else {
                    (p * t / (efficiency * 4184.0)).max(0.0)
                }
            })
            .collect();

        Self {
            view,
            points,
            raw_time_diff,
            geodesic,
            dist_diff,
            speed,
            power,
            pace,
            distance_km,
            calories,
        }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }
}

/// Watts from speed (m/s), as `TcxFileParser.estimate_power`. Python's `max(0.0, nan)` is
/// `0.0`, so an unknown speed gives zero rather than `NaN`.
fn estimate_power(view: ViewMode, v: f64) -> f64 {
    let watts = match view {
        ViewMode::Cycling => {
            let mass = BODY_WEIGHT + BIKE_WEIGHT;
            ROLLING_RESISTANCE * mass * GRAVITY * v + 0.5 * AIR_DENSITY * DRAG_AREA * v.powi(3)
        }
        ViewMode::Running => RUN_SHOE_RESISTANCE * BODY_WEIGHT * GRAVITY * v,
        ViewMode::Walking => WALK_SHOE_RESISTANCE * BODY_WEIGHT * GRAVITY * v,
        ViewMode::All => 0.0,
    };
    if watts > 0.0 { watts } else { 0.0 }
}

/// pandas `rolling(window, min_periods=1).mean()`: mean of the non-NaN values in the window.
fn rolling_mean(values: &[f64], window: usize) -> Vec<f64> {
    (0..values.len())
        .map(|i| {
            let start = (i + 1).saturating_sub(window);
            let valid: Vec<f64> = values[start..=i]
                .iter()
                .copied()
                .filter(|v| !v.is_nan())
                .collect();
            if valid.is_empty() {
                f64::NAN
            } else {
                valid.iter().sum::<f64>() / valid.len() as f64
            }
        })
        .collect()
}

/// pandas `interpolate()` (linear): fills interior gaps linearly and trailing gaps with the last
/// value; leading gaps stay `NaN`.
fn interpolate(values: &[f64]) -> Vec<f64> {
    let mut out = values.to_vec();
    let mut last: Option<usize> = None;
    for i in 0..values.len() {
        if values[i].is_nan() {
            continue;
        }
        if let Some(prev) = last
            && i > prev + 1
        {
            let (a, b) = (values[prev], values[i]);
            for (j, slot) in out.iter_mut().enumerate().take(i).skip(prev + 1) {
                *slot = a + (b - a) * (j - prev) as f64 / (i - prev) as f64;
            }
        }
        last = Some(i);
    }
    if let Some(prev) = last {
        for slot in out.iter_mut().skip(prev + 1) {
            *slot = values[prev];
        }
    }
    out
}

/// UTM zone (1–60) for the mean longitude, as `convert_to_utm` picks it.
pub fn utm_zone(points: &[RawPoint]) -> u8 {
    let mean = points.iter().map(|p| p.longitude).sum::<f64>() / points.len().max(1) as f64;
    (((mean + 180.0) / 6.0) as i64 + 1).clamp(1, 60) as u8
}

/// Projected (easting, northing) metres for each point, in the activity's UTM zone.
pub fn utm_coordinates(points: &[RawPoint]) -> Vec<(f64, f64)> {
    let zone = utm_zone(points);
    points
        .iter()
        .map(|p| {
            let (northing, easting, _) = utm::to_utm_wgs84(p.latitude, p.longitude, zone);
            (easting, northing)
        })
        .collect()
}

/// Cumulative km over UTM coordinates, the `Distance` column of `calculate_distance`.
fn utm_cumulative_km(points: &[RawPoint]) -> Vec<f64> {
    let xy = utm_coordinates(points);
    let mut total = 0.0;
    (0..xy.len())
        .map(|i| {
            if i > 0 {
                let (dx, dy) = (xy[i].0 - xy[i - 1].0, xy[i].1 - xy[i - 1].1);
                total += (dx * dx + dy * dy).sqrt() / 1000.0;
            }
            total
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(x, y)| (x.is_nan() && y.is_nan()) || (x - y).abs() < 1e-12)
    }

    #[test]
    fn interpolates_like_pandas() {
        let nan = f64::NAN;
        let out = interpolate(&[nan, 1.0, nan, nan, 4.0, nan]);
        assert!(same(&out, &[nan, 1.0, 2.0, 3.0, 4.0, 4.0]));
    }

    #[test]
    fn rolling_mean_skips_nan() {
        let nan = f64::NAN;
        let out = rolling_mean(&[nan, 1.0, 1.0, 4.0], 2);
        assert!(same(&out, &[nan, 1.0, 1.0, 2.5]));
    }

    #[test]
    fn unknown_speed_estimates_zero_power() {
        assert_eq!(estimate_power(ViewMode::Running, f64::NAN), 0.0);
        assert!((estimate_power(ViewMode::Running, 3.0) - 41.202).abs() < 1e-9);
    }

    #[test]
    fn picks_utm_zone_from_mean_longitude() {
        let point = |lon| RawPoint {
            time: chrono::DateTime::UNIX_EPOCH,
            latitude: 48.0,
            longitude: lon,
            elevation: 0.0,
            heart_rate: None,
            cadence: None,
            power: None,
        };
        assert_eq!(utm_zone(&[point(-3.8)]), 30);
        assert_eq!(utm_zone(&[point(13.4)]), 33);
    }
}
