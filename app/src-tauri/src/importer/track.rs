//! Per-trackpoint derived values, ported from `processing/tcx_file_parser.py` and
//! `processing/data_processing.py`. Missing values are `NaN`, as in the pandas original, so the
//! formulas read the same.

use geographiclib_rs::{Geodesic, InverseGeodesic};

use super::{grade, tcx::RawPoint};
use crate::activity_type::ViewMode;

const GRAVITY: f64 = 9.81;
/// Python's `16.6667`: s/m → min/km.
const PACE_FACTOR: f64 = 16.6667;
/// Body weight (kg) when the profile has none.
pub const DEFAULT_BODY_WEIGHT: f64 = 70.0;
/// Bike parameters of `TcxFileParser`, used to estimate missing cycling power.
const BIKE_WEIGHT: f64 = 8.0;
const ROLLING_RESISTANCE: f64 = 0.004;
const AIR_DENSITY: f64 = 1.225;
const DRAG_AREA: f64 = 0.27;
/// Share of the energy burned that a rider turns into power at the pedals.
const CYCLING_EFFICIENCY: f64 = 0.25;
/// Share of the energy burned (above resting) that a runner or walker turns into mechanical
/// power; gives the ~1 W/kg per m/s of running power meters.
const FOOT_EFFICIENCY: f64 = 0.25;
/// Oxygen at rest, ml/kg/min (1 MET).
const RESTING_OXYGEN: f64 = 3.5;
/// Energy per ml of oxygen: 5 kcal per litre, as in the ACSM equations.
const KCAL_PER_ML_OXYGEN: f64 = 0.005;
const JOULES_PER_KCAL: f64 = 4184.0;
/// Gaps longer than this (seconds) count as a pause.
pub const PAUSE_THRESHOLD: f64 = 10.0;

/// Oxygen cost of the ACSM running and walking equations, ml/kg per metre: horizontal, and per
/// metre climbed. `VO2 = 3.5 + horizontal × speed + vertical × speed × grade` (ml/kg/min, speed in
/// m/min); downhill is counted as flat.
fn oxygen_cost(view: ViewMode) -> Option<(f64, f64)> {
    match view {
        ViewMode::Running => Some((0.2, 0.9)),
        ViewMode::Walking => Some((0.1, 1.8)),
        ViewMode::Cycling | ViewMode::All => None,
    }
}

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
    /// m/s.
    pub speed: Vec<f64>,
    /// Measured watts, or estimated from speed and climb when missing.
    pub power: Vec<f64>,
    /// min/km, `NaN` outside the plausible range for the activity type.
    pub pace: Vec<f64>,
    /// Energy cost of the point's smoothed gradient over that of the flat
    /// ([`grade::cost_factor`]); `NaN` for rides.
    pub grade_factor: Vec<f64>,
    /// Grade-adjusted pace, min/km: the pace divided by [`Self::grade_factor`]. `NaN` where the
    /// pace is, for rides and for the point after a pause.
    pub gap: Vec<f64>,
    /// Cumulative km over UTM-projected coordinates.
    pub distance_km: Vec<f64>,
    /// kcal since the previous point.
    pub calories: Vec<f64>,
}

impl Track {
    /// `weight` is the body weight in kg.
    pub fn new(points: Vec<RawPoint>, view: ViewMode, weight: f64) -> Self {
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
        // Speed, pace and power use this; pauses, steps and calories use the raw gaps.
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
        // Zeros interpolated, as pandas' `replace(0, NaN).interpolate()` does, for speed and pace.
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

        // Metres climbed since the previous point; the activity's elevation gain sums the same.
        let rise: Vec<f64> = (0..n)
            .map(|i| {
                if i == 0 {
                    0.0
                } else {
                    (points[i].elevation - points[i - 1].elevation).max(0.0)
                }
            })
            .collect();

        let power: Vec<f64> = (0..n)
            .map(|i| {
                points[i].power.unwrap_or_else(|| {
                    let climb_rate = rise[i] / time_diff[i];
                    estimate_power(view, weight, speed[i], climb_rate)
                })
            })
            .collect();

        let (min_pace, max_pace, min_dist) = pace_limits(view);
        let pace: Vec<f64> = dist_diff
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

        let along: Vec<f64> = geodesic
            .iter()
            .scan(0.0, |total, d| {
                *total += d;
                Some(*total)
            })
            .collect();
        let elevation: Vec<f64> = points.iter().map(|p| p.elevation).collect();
        let grade_factor: Vec<f64> = grade::gradients(&along, &elevation)
            .into_iter()
            .map(|g| grade::cost_factor(view, g).unwrap_or(f64::NAN))
            .collect();
        let gap = (0..n)
            .map(|i| {
                if raw_time_diff[i] > PAUSE_THRESHOLD {
                    f64::NAN
                } else {
                    pace[i] / grade_factor[i]
                }
            })
            .collect();

        let calories = (0..n)
            .map(|i| {
                let kcal = match oxygen_cost(view) {
                    // ACSM equation summed per point: the speed term becomes distance and the
                    // grade term metres climbed. Measured watts don't change what a run costs.
                    // The raw distance, so standing still doesn't borrow the neighbours' steps.
                    Some((horizontal, vertical)) => {
                        let oxygen = horizontal * geodesic[i]
                            + vertical * rise[i]
                            + RESTING_OXYGEN * raw_time_diff[i] / 60.0;
                        weight * oxygen * KCAL_PER_ML_OXYGEN
                    }
                    // A pause isn't pedalled through at the power of the point after it.
                    None if raw_time_diff[i] > PAUSE_THRESHOLD => 0.0,
                    None => power[i] * raw_time_diff[i] / (CYCLING_EFFICIENCY * JOULES_PER_KCAL),
                };
                if kcal.is_nan() { 0.0 } else { kcal.max(0.0) }
            })
            .collect();

        Self {
            view,
            points,
            raw_time_diff,
            geodesic,
            speed,
            power,
            pace,
            grade_factor,
            gap,
            distance_km,
            calories,
        }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }
}

/// Watts from speed (m/s) and climb rate (m/s). Cycling uses rolling resistance and drag, as
/// `TcxFileParser.estimate_power`. Running and walking take the energy above resting from the
/// ACSM equations and [`FOOT_EFFICIENCY`] of it; Python's model (2 % of body weight against
/// speed, like a cart) came out about five times too low. An unknown speed gives zero rather
/// than `NaN`, as Python's `max(0.0, nan)` did.
fn estimate_power(view: ViewMode, weight: f64, v: f64, climb_rate: f64) -> f64 {
    let climb_rate = if climb_rate.is_finite() {
        climb_rate
    } else {
        0.0
    };
    let watts = match oxygen_cost(view) {
        Some((horizontal, vertical)) => {
            let oxygen_per_second = weight * (horizontal * v + vertical * climb_rate);
            oxygen_per_second * KCAL_PER_ML_OXYGEN * JOULES_PER_KCAL * FOOT_EFFICIENCY
        }
        None if view == ViewMode::Cycling => {
            let mass = weight + BIKE_WEIGHT;
            ROLLING_RESISTANCE * mass * GRAVITY * v + 0.5 * AIR_DENSITY * DRAG_AREA * v.powi(3)
        }
        None => 0.0,
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
        assert_eq!(estimate_power(ViewMode::Running, 70.0, f64::NAN, 0.0), 0.0);
    }

    #[test]
    fn running_power_is_about_one_watt_per_kg_and_metre_per_second() {
        // 68 kg at 2.36 m/s (6:47 min/km) on the flat.
        let flat = estimate_power(ViewMode::Running, 68.0, 2.36, 0.0);
        assert!((flat - 167.9).abs() < 0.1, "{flat}");
        // Climbing 0.05 m/s (a 2 % grade) on top: 68 × 0.9 × 0.05 ml/s.
        let climb = estimate_power(ViewMode::Running, 68.0, 2.36, 0.05) - flat;
        assert!((climb - 16.0).abs() < 0.1, "{climb}");
    }

    #[test]
    fn calories_follow_the_acsm_running_equation() {
        // 1 km in 6 minutes on the flat, one point every 6 s.
        let points: Vec<RawPoint> = (0..=60)
            .map(|i| RawPoint {
                time: chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(i * 6),
                latitude: 48.0 + i as f64 * 1000.0 / 60.0 / 111_195.0,
                longitude: -3.8,
                elevation: 50.0,
                heart_rate: None,
                cadence: None,
                power: None,
            })
            .collect();
        let track = Track::new(points, ViewMode::Running, 68.0);
        let kcal: f64 = track.calories.iter().sum();
        // 68 kg × (0.2 × 1000 m + 3.5 × 6 min) ml/kg × 5 kcal/l.
        let expected = 68.0 * (0.2 * 1000.0 + 3.5 * 6.0) * 0.005;
        assert!((kcal - expected).abs() < 1.0, "{kcal} vs {expected}");
    }

    /// Points 6 s and about 16.7 m apart heading north; `stops` lists the indices that repeat
    /// the previous position, `gap` the index that comes 600 s after the one before.
    fn line(stops: &[i64], gap: Option<i64>, power: Option<f64>) -> Vec<RawPoint> {
        let mut north = 0.0;
        (0..=60)
            .map(|i| {
                if i > 0 && !stops.contains(&i) {
                    north += 1000.0 / 60.0;
                }
                let late = gap.is_some_and(|g| i >= g);
                RawPoint {
                    time: chrono::DateTime::UNIX_EPOCH
                        + chrono::TimeDelta::seconds(i * 6 + if late { 594 } else { 0 }),
                    latitude: 48.0 + north / 111_195.0,
                    longitude: -3.8,
                    elevation: 50.0,
                    heart_rate: None,
                    cadence: Some(80.0),
                    power,
                }
            })
            .collect()
    }

    #[test]
    fn standing_still_burns_no_distance() {
        let kcal = |points| {
            Track::new(points, ViewMode::Running, 68.0)
                .calories
                .iter()
                .sum::<f64>()
        };
        let moving = kcal(line(&[], None, None));
        // Ten stops of 6 s each: 10 × 16.7 m less and the same time.
        let stopping = kcal(line(&[10, 11, 12, 13, 14, 30, 31, 32, 33, 34], None, None));
        let expected = 68.0 * 0.2 * 10.0 * 1000.0 / 60.0 * 0.005;
        assert!(
            (moving - stopping - expected).abs() < 0.1,
            "{moving} vs {stopping}"
        );
    }

    #[test]
    fn a_pause_is_not_pedalled_through() {
        let kcal = |points| {
            Track::new(points, ViewMode::Cycling, 68.0)
                .calories
                .iter()
                .sum::<f64>()
        };
        let steady = kcal(line(&[], None, Some(150.0)));
        let paused = kcal(line(&[], Some(30), Some(150.0)));
        // Only the 6 s step into the point after the pause is lost.
        let step = 150.0 * 6.0 / (CYCLING_EFFICIENCY * JOULES_PER_KCAL);
        assert!(
            (steady - paused - step).abs() < 1e-6,
            "{steady} vs {paused}"
        );
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
