//! Time spent in each heart-rate zone, after `ui/widget_heart_rate_zones.py`.
//!
//! The bounds come from `users::resolve_zones` for the user's zone method. They are upper
//! bounds, as the settings page and the zone labels present them. (The Python classifier used them as lower bounds, so a
//! heart rate was shown one zone too high.) Python also only looked at per-km segment averages;
//! this uses every trackpoint when the archived TCX file is available.

use serde::Serialize;
use ts_rs::TS;

use crate::{db::users::ZoneBounds, importer::track::Track};

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ZoneTime {
    /// 1–5.
    pub zone: u8,
    /// Lowest bpm in the zone; zone 1 starts at the method's floor.
    #[ts(type = "number")]
    pub min_bpm: i64,
    #[ts(type = "number")]
    pub max_bpm: i64,
    pub seconds: f64,
    /// Share of the time with a heart rate, 0–100.
    pub percent: f64,
}

/// Zone (1–5) of a heart rate for ascending upper bounds; above zone 5 still counts as 5.
pub fn zone_of(heart_rate: f64, bounds: &[i64; 5]) -> u8 {
    bounds
        .iter()
        .position(|bound| heart_rate <= *bound as f64)
        .map_or(5, |i| i as u8 + 1)
}

fn ranges(bounds: &ZoneBounds) -> [(i64, i64); 5] {
    let upper = &bounds.upper;
    std::array::from_fn(|i| {
        let min = if i == 0 {
            bounds.floor
        } else {
            upper[i - 1] + 1
        };
        (min, upper[i])
    })
}

fn summarise(bounds: &ZoneBounds, seconds: [f64; 5]) -> Vec<ZoneTime> {
    let total: f64 = seconds.iter().sum();
    ranges(bounds)
        .iter()
        .enumerate()
        .map(|(i, (min, max))| ZoneTime {
            zone: i as u8 + 1,
            min_bpm: *min,
            max_bpm: *max,
            seconds: seconds[i],
            percent: if total > 0.0 {
                seconds[i] / total * 100.0
            } else {
                0.0
            },
        })
        .collect()
}

/// Zone times from trackpoints: each point's heart rate counts for the time since the previous
/// point. Gaps over 30 s (pauses) are ignored.
pub fn from_track(track: &Track, bounds: &ZoneBounds) -> Vec<ZoneTime> {
    let mut seconds = [0.0; 5];
    for (point, gap) in track.points.iter().zip(&track.raw_time_diff).skip(1) {
        if let Some(hr) = point.heart_rate
            && hr > 0.0
            && *gap > 0.0
            && *gap <= 30.0
        {
            seconds[zone_of(hr, &bounds.upper) as usize - 1] += gap;
        }
    }
    summarise(bounds, seconds)
}

/// Fallback without a TCX file: `(average heart rate, seconds)` per segment, as in Python.
pub fn from_segments(segments: &[(f64, f64)], bounds: &ZoneBounds) -> Vec<ZoneTime> {
    let mut seconds = [0.0; 5];
    for (hr, duration) in segments {
        if *hr > 0.0 && *duration > 0.0 {
            seconds[zone_of(*hr, &bounds.upper) as usize - 1] += duration;
        }
    }
    summarise(bounds, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: [i64; 5] = [98, 114, 131, 147, 164];

    #[test]
    fn bounds_are_upper_limits() {
        assert_eq!(zone_of(90.0, &BOUNDS), 1);
        assert_eq!(zone_of(98.0, &BOUNDS), 1);
        assert_eq!(zone_of(99.0, &BOUNDS), 2);
        assert_eq!(zone_of(150.0, &BOUNDS), 5);
        assert_eq!(zone_of(147.0, &BOUNDS), 4);
        assert_eq!(zone_of(190.0, &BOUNDS), 5);
    }

    #[test]
    fn summarises_segments() {
        let bounds = ZoneBounds {
            floor: 82,
            upper: BOUNDS,
        };
        let zones = from_segments(&[(140.0, 300.0), (150.0, 100.0), (0.0, 50.0)], &bounds);
        assert_eq!(zones[0].min_bpm, 82);
        assert_eq!((zones[1].min_bpm, zones[1].max_bpm), (99, 114));
        assert_eq!(zones[3].seconds, 300.0);
        assert_eq!(zones[4].percent, 25.0);
    }
}
