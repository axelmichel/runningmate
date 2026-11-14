//! Fitness (CTL), fatigue (ATL) and form (TSB), ported from `ui/widget_fitness.py`.
//!
//! As in Python, the averages advance once per day *with* training (rest days don't decay
//! them), and the daily load is the mean segment load weighted by activity type.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

use crate::activity_type::ViewMode;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FitnessDay {
    /// "YYYY-MM-DD" (UTC, as Python grouped it).
    pub day: String,
    pub fitness: f64,
    pub fatigue: f64,
    pub form: f64,
}

/// Load weights per view.
pub fn weight(view: ViewMode) -> Option<f64> {
    match view {
        ViewMode::Running => Some(1.0),
        ViewMode::Walking => Some(0.5),
        ViewMode::Cycling => Some(0.8),
        ViewMode::All => None,
    }
}

/// Sums weighted loads per day from `(day, view, mean segment load)` rows.
pub fn daily_loads(rows: &[(String, ViewMode, Option<f64>)]) -> Vec<(String, f64)> {
    let mut days: BTreeMap<String, f64> = BTreeMap::new();
    for (day, view, load) in rows {
        if let Some(w) = weight(*view) {
            *days.entry(day.clone()).or_default() += load.unwrap_or(0.0) * w;
        }
    }
    days.into_iter().collect()
}

/// Exponential averages over 42 (fitness) and 7 (fatigue) training days, seeded with the first
/// day's load.
pub fn fitness_curve(loads: &[(String, f64)]) -> Vec<FitnessDay> {
    let Some((_, first)) = loads.first() else {
        return Vec::new();
    };
    let (mut ctl, mut atl) = (*first, *first);
    loads
        .iter()
        .map(|(day, load)| {
            ctl += (load - ctl) / 42.0;
            atl += (load - atl) / 7.0;
            FitnessDay {
                day: day.clone(),
                fitness: ctl,
                fatigue: atl,
                form: ctl - atl,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_and_sums_days() {
        let rows = [
            ("2025-05-01".into(), ViewMode::Running, Some(100.0)),
            ("2025-05-01".into(), ViewMode::Walking, Some(40.0)),
            ("2025-05-02".into(), ViewMode::Cycling, Some(50.0)),
            ("2025-05-02".into(), ViewMode::All, Some(999.0)),
        ];
        assert_eq!(
            daily_loads(&rows),
            [
                ("2025-05-01".to_string(), 120.0),
                ("2025-05-02".to_string(), 40.0)
            ]
        );
    }

    #[test]
    fn curve_starts_at_first_load() {
        let curve = fitness_curve(&[("a".into(), 70.0), ("b".into(), 0.0)]);
        assert_eq!(curve[0].fitness, 70.0);
        assert!((curve[1].fitness - (70.0 - 70.0 / 42.0)).abs() < 1e-12);
        assert!((curve[1].fatigue - 60.0).abs() < 1e-12);
        assert!(curve[1].form > 0.0);
    }
}
