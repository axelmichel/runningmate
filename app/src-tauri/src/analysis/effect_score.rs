//! Effect score (1–12) and recovery time, ported unchanged from `ui/widget_trainings_effect.py`:
//! the app's own heuristic from split averages compared with the last 90 days, shown beside the
//! EPOC-based training effect (`training_effect.rs`).

use serde::Serialize;
use ts_rs::TS;

use crate::activity_type::ViewMode;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EffectScore {
    /// 1–12.
    pub score: u8,
    /// Suggested rest in hours.
    pub recovery_hours: f64,
}

/// Averages of the segment columns used by the formula.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentAverages {
    pub heart_rate: f64,
    pub power: f64,
    pub speed: f64,
    pub pace: f64,
}

/// Raw load of one segment (or activity average): HR × power for rides, HR × pace otherwise,
/// divided by speed + 1.
pub fn load(view: ViewMode, s: &SegmentAverages) -> f64 {
    if view == ViewMode::Cycling {
        s.heart_rate * s.power / (s.speed + 1.0)
    } else {
        s.heart_rate * s.pace / (s.speed + 1.0)
    }
}

/// Mean load of earlier segments (speed floored at 0.1), at least 10; 10 without history.
pub fn baseline(view: ViewMode, history: &[SegmentAverages]) -> f64 {
    let loads: Vec<f64> = history
        .iter()
        .map(|s| {
            load(
                view,
                &SegmentAverages {
                    speed: s.speed.max(0.1),
                    ..*s
                },
            )
        })
        .filter(|l| !l.is_nan())
        .collect();
    if loads.is_empty() {
        return 10.0;
    }
    (loads.iter().sum::<f64>() / loads.len() as f64).max(10.0)
}

pub fn score(view: ViewMode, current: &SegmentAverages, baseline: f64) -> u8 {
    let values = [
        current.heart_rate,
        current.power,
        current.speed,
        current.pace,
        baseline,
    ];
    if values.iter().any(|v| v.is_nan()) {
        return 1;
    }
    let current_load = load(view, current);
    if current_load == 0.0 {
        return 1;
    }
    let zone_factor = 1.0 + (current.heart_rate - 120.0) / 20.0;
    let effect = ((current_load * zone_factor).max(0.0) / (baseline.max(0.1) + 1.0)).powf(0.8);
    (effect * 6.0).round_ties_even().clamp(1.0, 12.0) as u8
}

/// `6 · e^(0.28 · score)` hours.
pub fn recovery_hours(score: u8) -> f64 {
    6.0 * (0.28 * f64::from(score)).exp()
}

pub fn effect_score(
    view: ViewMode,
    current: &SegmentAverages,
    history: &[SegmentAverages],
) -> EffectScore {
    let score = score(view, current, baseline(view, history));
    EffectScore {
        score,
        recovery_hours: recovery_hours(score),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn averages(heart_rate: f64, pace: f64, speed: f64) -> SegmentAverages {
        SegmentAverages {
            heart_rate,
            power: 160.0,
            speed,
            pace,
        }
    }

    #[test]
    fn matches_python_formula() {
        // HR 147, pace 6.6, speed 2.5: load = 147 * 6.6 / 3.5 = 277.2; zone factor 2.35;
        // (277.2 * 2.35 / (250 + 1))^0.8 * 6 = 2.838… * … → rounded.
        let current = averages(147.0, 6.6, 2.5);
        let history = [averages(140.0, 6.5, 2.6), averages(150.0, 6.0, 2.8)];
        let base = baseline(ViewMode::Running, &history);
        let expected_base = (140.0 * 6.5 / 3.6 + 150.0 * 6.0 / 3.8) / 2.0;
        assert!((base - expected_base).abs() < 1e-9);
        let effect = ((147.0 * 6.6 / 3.5 * (1.0 + 27.0 / 20.0)) / (expected_base + 1.0)).powf(0.8);
        assert_eq!(
            score(ViewMode::Running, &current, base),
            (effect * 6.0).round().clamp(1.0, 12.0) as u8
        );
    }

    #[test]
    fn falls_back_without_data() {
        assert_eq!(baseline(ViewMode::Running, &[]), 10.0);
        let missing = averages(f64::NAN, 6.0, 2.5);
        assert_eq!(score(ViewMode::Running, &missing, 10.0), 1);
        assert!((recovery_hours(1) - 6.0 * 0.28f64.exp()).abs() < 1e-12);
    }
}
