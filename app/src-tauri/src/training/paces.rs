//! Training paces from a VDOT, after Jack Daniels' Running Formula: each kind of run is held at a
//! share of VO2max, and the oxygen cost formula of [`cardio::vdot`] gives the speed for it.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::analysis::cardio;

/// A pace range in s/km; `fast` is the lower number.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaceRange {
    pub fast: f64,
    pub slow: f64,
}

impl PaceRange {
    /// A range of `spread` s/km either side of `pace`.
    pub fn around(pace: f64, spread: f64) -> Self {
        Self {
            fast: pace - spread,
            slow: pace + spread,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Paces {
    pub vdot: f64,
    pub easy: PaceRange,
    pub marathon: PaceRange,
    pub threshold: PaceRange,
    pub interval: PaceRange,
    pub repetition: PaceRange,
}

/// Pace (s/km) at which the oxygen cost is `share` of `vdot`.
fn pace_at(vdot: f64, share: f64) -> f64 {
    // cost = -4.60 + 0.182258 v + 0.000104 v², v in m/min.
    let cost = vdot * share + 4.60;
    let (a, b) = (0.000_104, 0.182_258);
    let speed = (-b + (b * b + 4.0 * a * cost).sqrt()) / (2.0 * a);
    60_000.0 / speed
}

/// Time (s) a runner with `vdot` needs for `km` at full effort, found by bisection.
pub fn race_time(vdot: f64, km: f64) -> f64 {
    let metres = km * 1000.0;
    // Between 2:00 and 12:00 per km covers every plausible VDOT.
    let (mut fast, mut slow) = (km * 120.0, km * 720.0);
    for _ in 0..60 {
        let middle = (fast + slow) / 2.0;
        // A longer time means a lower VDOT; outside the plausible range counts as too slow.
        match cardio::vdot(metres, middle) {
            Some(value) if value > vdot => fast = middle,
            _ => slow = middle,
        }
    }
    (fast + slow) / 2.0
}

/// Daniels' paces for `vdot`.
pub fn paces(vdot: f64) -> Paces {
    let marathon = race_time(vdot, 42.195) / 42.195;
    Paces {
        vdot,
        easy: PaceRange {
            fast: pace_at(vdot, 0.70),
            slow: pace_at(vdot, 0.62),
        },
        marathon: PaceRange::around(marathon, 5.0),
        threshold: PaceRange {
            fast: pace_at(vdot, 0.89),
            slow: pace_at(vdot, 0.86),
        },
        interval: PaceRange {
            fast: pace_at(vdot, 0.99),
            slow: pace_at(vdot, 0.96),
        },
        repetition: PaceRange {
            fast: pace_at(vdot, 1.06),
            slow: pace_at(vdot, 1.03),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mmss(seconds: f64) -> String {
        let seconds = seconds.round() as i64;
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }

    #[test]
    fn match_daniels_tables() {
        // VDOT 50: E 5:07–5:38, M 4:31, T 4:15, I 3:55, R 3:40 per km in Daniels' tables.
        let p = paces(50.0);
        assert_eq!(mmss(p.easy.fast), "5:07");
        assert_eq!(mmss(p.easy.slow), "5:38");
        assert!(
            (p.marathon.fast + 5.0 - 271.0).abs() < 4.0,
            "{}",
            mmss(p.marathon.fast)
        );
        assert!(
            (p.threshold.fast - 253.0).abs() < 4.0,
            "{}",
            mmss(p.threshold.fast)
        );
        assert!(
            (p.interval.fast - 233.0).abs() < 4.0,
            "{}",
            mmss(p.interval.fast)
        );
        assert!(
            (p.repetition.fast - 220.0).abs() < 5.0,
            "{}",
            mmss(p.repetition.fast)
        );
    }

    #[test]
    fn race_time_inverts_vdot() {
        // Daniels: VDOT 50 runs 5 km in 19:57, VDOT 40 runs 10 km in 50:03.
        assert!((race_time(50.0, 5.0) - 1197.0).abs() < 5.0);
        assert!((race_time(40.0, 10.0) - 3003.0).abs() < 10.0);
    }
}
