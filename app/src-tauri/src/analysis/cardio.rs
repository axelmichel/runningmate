//! Cardio fitness: a VO2max estimate from run times, rated against people of the same age and
//! sex, as Garmin and Apple do it.
//!
//! RunningMate has no lab or watch measurement, so the estimate uses Jack Daniels' and Jimmy
//! Gilbert's VDOT formula (oxygen cost of the pace, divided by the share of VO2max that can be
//! held for that long). Training runs aren't all-out efforts, so the 90th percentile of the
//! period's runs is taken: it reflects the harder runs without trusting a single outlier, e.g.
//! one with a wrongly recorded pause.

use serde::Serialize;
use ts_rs::TS;

use crate::db::users::Gender;

/// Runs shorter than this say little about endurance.
pub const MIN_RUN_KM: f64 = 3.0;
/// Fewer runs than this give no estimate.
pub const MIN_RUNS: usize = 3;
/// Results outside this range are recording errors rather than fitness.
const PLAUSIBLE: std::ops::RangeInclusive<f64> = 15.0..=85.0;

/// VO2max (ml/kg/min) for covering `metres` in `seconds` at full effort.
pub fn vdot(metres: f64, seconds: f64) -> Option<f64> {
    if metres <= 0.0 || seconds <= 0.0 {
        return None;
    }
    let minutes = seconds / 60.0;
    let speed = metres / minutes;
    let cost = -4.60 + 0.182_258 * speed + 0.000_104 * speed * speed;
    let share = 0.8
        + 0.189_439_3 * (-0.012_778 * minutes).exp()
        + 0.298_955_8 * (-0.193_260_5 * minutes).exp();
    Some(cost / share).filter(|v| PLAUSIBLE.contains(v))
}

/// The 90th percentile of the per-run values; `None` with fewer than [`MIN_RUNS`].
pub fn estimate(mut values: Vec<f64>) -> Option<f64> {
    if values.len() < MIN_RUNS {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let index = ((values.len() - 1) as f64 * 0.9).round() as usize;
    Some(values[index])
}

/// Garmin's five levels, from the Cooper Institute percentiles (0–40, 40, 60, 80, 95).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CardioLevel {
    Poor,
    Fair,
    Good,
    Excellent,
    Superior,
}

/// Lower bounds of Fair, Good, Excellent and Superior for ages 20–29 … 70–79.
/// Reprinted in Garmin's manuals with permission from The Cooper Institute.
const MALE: [[f64; 4]; 6] = [
    [41.7, 45.4, 51.1, 55.4],
    [40.5, 44.0, 48.3, 54.0],
    [38.5, 42.4, 46.4, 52.5],
    [35.6, 39.2, 43.4, 48.9],
    [32.3, 35.5, 39.5, 45.7],
    [29.4, 32.3, 36.7, 42.1],
];
const FEMALE: [[f64; 4]; 6] = [
    [36.1, 39.5, 43.9, 49.6],
    [34.4, 37.8, 42.4, 47.4],
    [33.0, 36.3, 39.7, 45.3],
    [30.1, 33.0, 36.7, 41.1],
    [27.5, 30.0, 33.0, 37.8],
    [25.9, 28.1, 30.9, 36.7],
];

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardioRating {
    pub level: CardioLevel,
    /// The compared age group, e.g. 50 for 50–59. Ages outside 20–79 use the nearest group.
    #[ts(type = "number")]
    pub age_group: i64,
    pub gender: Gender,
    /// Lower bounds of Fair, Good, Excellent and Superior for this group.
    pub bounds: [f64; 4],
}

pub fn rating(vo2max: f64, age: i64, gender: Gender) -> CardioRating {
    let group = ((age.clamp(20, 79) - 20) / 10) as usize;
    let bounds = match gender {
        Gender::Male => MALE[group],
        Gender::Female => FEMALE[group],
    };
    let level = match bounds.iter().filter(|bound| vo2max >= **bound).count() {
        0 => CardioLevel::Poor,
        1 => CardioLevel::Fair,
        2 => CardioLevel::Good,
        3 => CardioLevel::Excellent,
        _ => CardioLevel::Superior,
    };
    CardioRating {
        level,
        age_group: 20 + group as i64 * 10,
        gender,
        bounds,
    }
}

/// Seconds of a pause stored as "HH:MM:SS" or "MM:SS"; 0 when empty or unreadable.
pub fn pause_seconds(text: &str) -> f64 {
    text.trim()
        .split(':')
        .try_fold(0.0, |total, part| {
            part.parse::<f64>().map(|n| total * 60.0 + n)
        })
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdot_matches_daniels_tables() {
        // Daniels' tables: VDOT 50 runs 5 km in 19:57, VDOT 40 runs 10 km in 50:03.
        assert!((vdot(5_000.0, 19.0 * 60.0 + 57.0).unwrap() - 50.0).abs() < 0.2);
        assert!((vdot(10_000.0, 50.0 * 60.0 + 3.0).unwrap() - 40.0).abs() < 0.2);
        assert_eq!(
            vdot(5_000.0, 600.0),
            None,
            "5 km in 10 minutes is a recording error"
        );
    }

    #[test]
    fn estimate_takes_the_90th_percentile() {
        assert_eq!(estimate(vec![30.0, 31.0]), None);
        let values = (0..11).map(|i| 30.0 + i as f64).collect();
        assert_eq!(estimate(values), Some(39.0));
    }

    #[test]
    fn rates_by_age_group_and_sex() {
        let man = rating(40.0, 58, Gender::Male);
        assert_eq!((man.level, man.age_group), (CardioLevel::Good, 50));
        assert_eq!(
            rating(40.0, 58, Gender::Female).level,
            CardioLevel::Excellent
        );
        assert_eq!(rating(60.0, 25, Gender::Male).level, CardioLevel::Superior);
        assert_eq!(rating(20.0, 85, Gender::Female).age_group, 70);
        assert_eq!(rating(20.0, 85, Gender::Female).level, CardioLevel::Poor);
    }

    #[test]
    fn reads_pauses() {
        assert_eq!(pause_seconds("00:21:22"), 1282.0);
        assert_eq!(pause_seconds("03:09"), 189.0);
        assert_eq!(pause_seconds(""), 0.0);
    }
}
