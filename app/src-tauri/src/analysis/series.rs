//! Per-point series for the statistics charts and the map, downsampled for the frontend.

use serde::Serialize;
use ts_rs::TS;

use crate::importer::track::Track;

/// Upper bound on points sent to the frontend; enough for smooth charts and map lines.
const MAX_POINTS: usize = 2000;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrackPoint {
    /// Cumulative km.
    pub distance: f64,
    /// Seconds since the start.
    pub elapsed: f64,
    pub latitude: f64,
    pub longitude: f64,
    /// Metres.
    pub elevation: f64,
    pub heart_rate: Option<f64>,
    /// min/km, `None` where implausible for the activity type.
    pub pace: Option<f64>,
    /// Steps per minute (both feet), `None` without cadence or when standing still.
    pub cadence: Option<f64>,
}

pub fn series(track: &Track) -> Vec<TrackPoint> {
    let n = track.len();
    let step = n.div_ceil(MAX_POINTS).max(1);
    let start = track.points.first().map(|p| p.time);
    (0..n)
        .filter(|i| i % step == 0 || *i == n - 1)
        .map(|i| {
            let point = &track.points[i];
            let pace = track.pace[i];
            TrackPoint {
                distance: track.distance_km[i],
                elapsed: start.map_or(0.0, |s| (point.time - s).as_seconds_f64()),
                latitude: point.latitude,
                longitude: point.longitude,
                elevation: point.elevation,
                heart_rate: point.heart_rate,
                pace: (!pace.is_nan()).then_some(pace),
                // TCX cadence counts one foot.
                cadence: point.cadence.filter(|c| *c > 0.0).map(|c| c * 2.0),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{activity_type::ViewMode, importer::tcx::RawPoint};

    #[test]
    fn cadence_counts_both_feet_and_drops_standstills() {
        let points: Vec<RawPoint> = [Some(80.0), Some(0.0), None]
            .into_iter()
            .enumerate()
            .map(|(i, cadence)| RawPoint {
                time: chrono::DateTime::UNIX_EPOCH + chrono::TimeDelta::seconds(i as i64 * 6),
                latitude: 48.0 + i as f64 * 1e-4,
                longitude: -3.8,
                elevation: 50.0,
                heart_rate: None,
                cadence,
                power: None,
            })
            .collect();
        let cadence: Vec<_> = series(&Track::new(points, ViewMode::Running, 70.0))
            .into_iter()
            .map(|p| p.cadence)
            .collect();
        assert_eq!(cadence, [Some(160.0), None, None]);
    }
}
