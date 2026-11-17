//! Grade-adjusted pace (GAP): the pace a run or walk would have had on the flat for the same
//! effort, from the energy cost of moving on a gradient measured by Minetti et al. (2002),
//! "Energy cost of walking and running at extreme uphill and downhill slopes", J Appl Physiol
//! 93: 1039–1046. Rides get none.
//!
//! The gradient of each point is the slope of a straight line fitted (least squares) to the
//! elevation over [`GRADIENT_WINDOW`] metres of track around it: per-point gradients of GPS or
//! barometric elevation are mostly noise.

use crate::activity_type::ViewMode;

/// Gradients beyond this (as a fraction, 0.45 = 45 %) are clamped: the range Minetti measured.
pub const MAX_GRADIENT: f64 = 0.45;
/// Metres of track the gradient of a point is fitted over, centred on it.
pub const GRADIENT_WINDOW: f64 = 100.0;
/// A window spanning less distance than this (a track shorter than it) counts as flat.
const MIN_SPAN: f64 = 20.0;

/// Coefficients of Minetti's fifth-degree polynomials, highest power first: energy cost in J
/// per kg and metre for the gradient `i` as a fraction (equations 1 and 2 of the paper).
const RUNNING: [f64; 6] = [155.4, -30.4, -43.3, 46.3, 19.5, 3.6];
const WALKING: [f64; 6] = [280.5, -58.7, -76.8, 51.9, 19.6, 2.5];

fn coefficients(view: ViewMode) -> Option<&'static [f64; 6]> {
    match view {
        ViewMode::Running => Some(&RUNNING),
        ViewMode::Walking => Some(&WALKING),
        ViewMode::Cycling | ViewMode::All => None,
    }
}

/// Energy cost (J/kg/m) of running or walking on `gradient`, clamped to ±[`MAX_GRADIENT`];
/// `None` for rides.
pub fn energy_cost(view: ViewMode, gradient: f64) -> Option<f64> {
    let i = gradient.clamp(-MAX_GRADIENT, MAX_GRADIENT);
    coefficients(view).map(|c| c.iter().fold(0.0, |sum, k| sum * i + k))
}

/// How much more (above 1) or less (below 1) a metre on `gradient` costs than one on the flat:
/// `C(i) / C(0)`. The grade-adjusted speed is the speed times this, the grade-adjusted pace
/// the pace divided by it.
pub fn cost_factor(view: ViewMode, gradient: f64) -> Option<f64> {
    Some(energy_cost(view, gradient)? / energy_cost(view, 0.0)?)
}

/// Gradient (fraction, uphill positive) at each point: the least-squares slope of elevation
/// over distance within [`GRADIENT_WINDOW`] metres centred on the point, clamped to
/// ±[`MAX_GRADIENT`]. `distance` is cumulative metres, never decreasing.
pub fn gradients(distance: &[f64], elevation: &[f64]) -> Vec<f64> {
    let n = distance.len().min(elevation.len());
    let half = GRADIENT_WINDOW / 2.0;
    let (mut lo, mut hi) = (0, 0);
    (0..n)
        .map(|i| {
            let centre = distance[i];
            while distance[lo] < centre - half {
                lo += 1;
            }
            while hi + 1 < n && distance[hi + 1] <= centre + half {
                hi += 1;
            }
            if distance[hi] - distance[lo] < MIN_SPAN {
                return 0.0;
            }
            // Distances relative to the point, so the sums stay small.
            let window = lo..=hi;
            let count = (hi - lo + 1) as f64;
            let mean_x = window.clone().map(|j| distance[j] - centre).sum::<f64>() / count;
            let mean_y = window.clone().map(|j| elevation[j]).sum::<f64>() / count;
            let (mut sxy, mut sxx) = (0.0, 0.0);
            for j in window {
                let x = distance[j] - centre - mean_x;
                sxy += x * (elevation[j] - mean_y);
                sxx += x * x;
            }
            if sxx > 0.0 {
                (sxy / sxx).clamp(-MAX_GRADIENT, MAX_GRADIENT)
            } else {
                0.0
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costs_match_minetti() {
        let run = |i| energy_cost(ViewMode::Running, i).unwrap();
        assert!((run(0.0) - 3.6).abs() < 1e-12);
        // 10 % up: 0.001554 − 0.00304 − 0.0433 + 0.463 + 1.95 + 3.6.
        assert!((run(0.1) - 5.968214).abs() < 1e-6, "{}", run(0.1));
        // Cheapest around −18 %, steeper downhill costs more again.
        assert!(run(-0.18) < run(-0.1) && run(-0.18) < run(-0.3));
        assert!(run(-0.45) > run(0.0));
        assert!((energy_cost(ViewMode::Walking, 0.0).unwrap() - 2.5).abs() < 1e-12);
        // Clamped beyond the measured range.
        assert_eq!(run(0.8), run(MAX_GRADIENT));
        assert_eq!(energy_cost(ViewMode::Cycling, 0.1), None);
    }

    #[test]
    fn fits_the_slope_through_noise() {
        // 5 m steps on a 5 % slope with ±2 m of alternating noise: 80 % per step.
        let distance: Vec<f64> = (0..200).map(|i| i as f64 * 5.0).collect();
        let elevation: Vec<f64> = distance
            .iter()
            .enumerate()
            .map(|(i, d)| 0.05 * d + if i % 2 == 0 { 2.0 } else { -2.0 })
            .collect();
        let gradients = gradients(&distance, &elevation);
        for g in &gradients[10..190] {
            assert!((g - 0.05).abs() < 0.01, "{g}");
        }
    }

    #[test]
    fn clamps_and_handles_short_tracks() {
        let cliff = gradients(&[0.0, 50.0, 100.0], &[0.0, 50.0, 100.0]);
        assert!(cliff.iter().all(|g| *g == MAX_GRADIENT));
        assert_eq!(gradients(&[0.0, 5.0], &[0.0, 3.0]), [0.0, 0.0]);
        // Standing still in the window doesn't stop the fit.
        let still = gradients(&[0.0, 30.0, 30.0, 30.0, 60.0], &[0.0, 3.0, 3.0, 3.0, 6.0]);
        assert!(still.iter().all(|g| (g - 0.1).abs() < 1e-9), "{still:?}");
    }

    use crate::importer::{metrics, segments, tcx::RawPoint, track::Track};

    /// 3 km north in steps of 1000/60 m every 6 s (6:00 min/km), point `i` at `elevation(i)`;
    /// `pause_at` comes 600 s after the point before.
    fn track_over(view: ViewMode, elevation: impl Fn(i64) -> f64, pause_at: Option<i64>) -> Track {
        let points = (0..=180)
            .map(|i| {
                let late = pause_at.is_some_and(|p| i >= p);
                RawPoint {
                    time: chrono::DateTime::UNIX_EPOCH
                        + chrono::TimeDelta::seconds(i * 6 + if late { 600 } else { 0 }),
                    latitude: 48.0 + i as f64 * STEP / 111_195.0,
                    longitude: -3.8,
                    elevation: elevation(i),
                    heart_rate: None,
                    cadence: None,
                    power: None,
                }
            })
            .collect();
        Track::new(points, view, 70.0)
    }

    const STEP: f64 = 1000.0 / 60.0;

    /// [`track_over`] a steady `grade`, with `noise` metres added and taken off in turn.
    fn track(view: ViewMode, grade: f64, noise: f64, pause_at: Option<i64>) -> Track {
        let elevation =
            |i: i64| 200.0 + grade * i as f64 * STEP + if i % 2 == 0 { noise } else { -noise };
        track_over(view, elevation, pause_at)
    }

    fn ratio(track: &Track) -> f64 {
        let m = metrics::compute(track);
        m.avg_gap.unwrap() / m.avg_pace
    }

    #[test]
    fn flat_gap_is_the_pace() {
        let flat = track(ViewMode::Running, 0.0, 0.0, None);
        for (gap, pace) in flat.gap.iter().zip(&flat.pace) {
            assert!((gap.is_nan() && pace.is_nan()) || gap == pace);
        }
        assert!((ratio(&flat) - 1.0).abs() < 1e-12);
        for s in segments::split(&flat) {
            assert!((s.avg_gap.unwrap() - s.avg_pace).abs() < 1e-12);
        }
    }

    #[test]
    fn uphill_gap_is_faster_than_the_pace() {
        let up = track(ViewMode::Running, 0.05, 0.0, None);
        // The geodesic steps are a little longer than 1000/60 m, so the gradient a little less.
        let expected = 1.0 / cost_factor(ViewMode::Running, 0.05).unwrap();
        assert!((ratio(&up) - expected).abs() < 1e-3, "{}", ratio(&up));
        assert!(ratio(&up) < 0.85);
        // Walking uphill costs relatively more than running.
        assert!(
            cost_factor(ViewMode::Walking, 0.05).unwrap()
                > cost_factor(ViewMode::Running, 0.05).unwrap()
        );
    }

    #[test]
    fn downhill_is_slower_until_it_gets_steep() {
        // Gentle downhill is cheaper than the flat: the flat equivalent is slower.
        assert!(ratio(&track(ViewMode::Running, -0.1, 0.0, None)) > 1.5);
        // Braking down 40 % costs more than the flat again.
        assert!(ratio(&track(ViewMode::Running, -0.4, 0.0, None)) < 1.0);
    }

    #[test]
    fn elevation_noise_is_smoothed_out() {
        // ±3 m every 17 m would be ±36 % gradients point by point.
        let noisy = track(ViewMode::Running, 0.0, 3.0, None);
        assert!((ratio(&noisy) - 1.0).abs() < 0.02, "{}", ratio(&noisy));
    }

    #[test]
    fn the_step_over_a_pause_has_no_gap() {
        let paused = track(ViewMode::Running, 0.05, 0.0, Some(90));
        assert!(paused.gap[90].is_nan());
        assert!(paused.gap[89].is_finite() && paused.gap[91].is_finite());
    }

    #[test]
    fn splits_get_gap() {
        let up = track(ViewMode::Running, 0.05, 0.0, None);
        let splits = segments::split(&up);
        assert_eq!(splits.len(), 3);
        let factor = cost_factor(ViewMode::Running, 0.05).unwrap();
        // The ends of the track see less of the slope in their window, so look at the middle.
        let middle = &splits[1];
        let gap = middle.avg_gap.unwrap();
        assert!((gap * factor / middle.avg_pace - 1.0).abs() < 1e-3, "{gap}");
    }

    #[test]
    fn a_hilly_loop_is_faster_adjusted() {
        // 1.5 km up 5 %, 1.5 km back down: the climb costs more than the descent saves.
        let loop_ = track_over(
            ViewMode::Running,
            |i| 200.0 + 0.05 * STEP * (90 - (i - 90).abs()) as f64,
            None,
        );
        let m = metrics::compute(&loop_);
        let gap = m.avg_gap.unwrap();
        assert!(gap < m.avg_pace * 0.99, "{gap} vs {}", m.avg_pace);
        // And the splits agree: their time over their flat distances.
        let splits = segments::split(&loop_);
        let minutes: f64 = splits.iter().map(|s| s.avg_pace * s.distance).sum();
        let flat: f64 = splits
            .iter()
            .map(|s| s.avg_pace * s.distance / s.avg_gap.unwrap())
            .sum();
        assert!(
            (minutes / flat / gap - 1.0).abs() < 0.01,
            "{}",
            minutes / flat
        );
    }

    #[test]
    fn rides_get_none() {
        let ride = track(ViewMode::Cycling, 0.05, 0.0, None);
        assert!(ride.gap.iter().all(|g| g.is_nan()));
        assert_eq!(metrics::compute(&ride).avg_gap, None);
        assert!(segments::split(&ride).iter().all(|s| s.avg_gap.is_none()));
    }
}
