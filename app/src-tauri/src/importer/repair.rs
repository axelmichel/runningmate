//! Route fix editor: finds GPS spikes and dropouts in an activity's samples and replaces the
//! positions of a stretch with a route drawn by hand. The repaired track is archived as TCX and
//! the activity recomputed from it; the original archive is kept for a revert.

use chrono::{DateTime, TimeDelta, Utc};
use geographiclib_rs::{Geodesic, InverseGeodesic};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::tcx::{RawSample, TcxActivity};
use super::track::pace_limits;
use crate::activity_type::ViewMode;
use crate::error::{AppError, AppResult};

/// One sample as the editor sees it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FixSample {
    /// Unix time in milliseconds.
    #[ts(type = "number")]
    pub time: i64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub elevation: Option<f64>,
    pub heart_rate: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FixIssueKind {
    /// Positions far off the route: steps faster than the sport allows.
    Spike,
    /// Samples without a position, or a long jump in time and place.
    Gap,
    /// No sample has a position.
    NoGps,
}

/// A stretch to redraw, as sample indices. `start` and `end` are the good samples around it
/// (the anchors the drawn route starts and ends at); for a stretch at the very start or end of
/// the activity, or without any GPS, they are the first or last sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FixIssue {
    pub kind: FixIssueKind,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FixSource {
    pub samples: Vec<FixSample>,
    pub issues: Vec<FixIssue>,
    /// Whether the activity was fixed before, so it can be reset to its original file.
    pub has_original: bool,
    /// `[latitude, longitude]` to centre the map on when no sample has a position.
    pub center: Option<(f64, f64)>,
    /// Running, walking or cycling, for routing along the right kind of paths.
    pub view: ViewMode,
}

/// A drawn route for the samples from `from` to `to` (Unix milliseconds, both included). The
/// first and last vertex are where the samples at `from` and `to` end up, so the editor starts
/// and ends the route at the anchors.
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FixEdit {
    #[ts(type = "number")]
    pub from: i64,
    #[ts(type = "number")]
    pub to: i64,
    /// `[latitude, longitude]` vertices.
    pub path: Vec<(f64, f64)>,
}

/// A step faster than this many times the fastest pace of the sport is a GPS spike.
const SPIKE_FACTOR: f64 = 1.5;
/// Fast steps closer together than this belong to one spike (the jump out and back).
const SPIKE_MERGE: TimeDelta = TimeDelta::seconds(120);
/// A time gap between two positions longer than this, covering more than [`GAP_METRES`], is a
/// dropout rather than a pause.
const GAP_TIME: TimeDelta = TimeDelta::seconds(30);
const GAP_METRES: f64 = 200.0;
/// Within a redrawn stretch, gaps longer than this get a sample every second, so the stretch
/// counts as moving and not as a pause (below `track::PAUSE_THRESHOLD`).
const FILL_GAP: TimeDelta = TimeDelta::seconds(5);

fn metres(a: (f64, f64), b: (f64, f64)) -> f64 {
    Geodesic::wgs84().inverse(a.0, a.1, b.0, b.1)
}

pub fn source(activity: &TcxActivity, has_original: bool) -> FixSource {
    FixSource {
        samples: activity
            .samples
            .iter()
            .map(|s| FixSample {
                time: s.time.timestamp_millis(),
                lat: s.position.map(|p| p.0),
                lon: s.position.map(|p| p.1),
                elevation: s.elevation,
                heart_rate: s.heart_rate,
            })
            .collect(),
        issues: issues(activity),
        has_original,
        center: None,
        view: ViewMode::from_activity_type(&activity.sport),
    }
}

/// Spikes, gaps and missing GPS, in time order.
pub fn issues(activity: &TcxActivity) -> Vec<FixIssue> {
    let samples = &activity.samples;
    let last = samples.len().saturating_sub(1);
    let positioned: Vec<usize> = (0..samples.len())
        .filter(|&i| samples[i].position.is_some())
        .collect();
    if samples.len() < 2 {
        return Vec::new();
    }
    if positioned.is_empty() {
        return vec![FixIssue {
            kind: FixIssueKind::NoGps,
            start: 0,
            end: last,
        }];
    }

    let mut found = Vec::new();
    // Runs of samples without a position.
    let mut i = 0;
    while i < samples.len() {
        if samples[i].position.is_some() {
            i += 1;
            continue;
        }
        let run_start = i;
        while i < samples.len() && samples[i].position.is_none() {
            i += 1;
        }
        found.push(FixIssue {
            kind: FixIssueKind::Gap,
            start: run_start.saturating_sub(1),
            end: i.min(last),
        });
    }

    let view = ViewMode::from_activity_type(&activity.sport);
    let max_speed = 1000.0 / (pace_limits(view).0 * 60.0) * SPIKE_FACTOR;
    let mut spike: Option<FixIssue> = None;
    for pair in positioned.windows(2) {
        let (a, b) = (&samples[pair[0]], &samples[pair[1]]);
        let distance = metres(a.position.unwrap(), b.position.unwrap());
        let dt = b.time - a.time;
        let seconds = dt.num_milliseconds() as f64 / 1000.0;
        if dt > GAP_TIME && distance > GAP_METRES && pair[1] == pair[0] + 1 {
            found.push(FixIssue {
                kind: FixIssueKind::Gap,
                start: pair[0],
                end: pair[1],
            });
            continue;
        }
        if seconds <= 0.0 || distance / seconds <= max_speed {
            continue;
        }
        spike = Some(match spike {
            Some(open) if a.time - samples[open.end].time <= SPIKE_MERGE => FixIssue {
                end: pair[1],
                ..open
            },
            open => {
                if let Some(open) = open {
                    found.push(open);
                }
                FixIssue {
                    kind: FixIssueKind::Spike,
                    start: pair[0],
                    end: pair[1],
                }
            }
        });
    }
    found.extend(spike);
    found.sort_by_key(|issue| (issue.start, issue.end));
    found
}

/// Cumulative fractions of `path`'s length at each vertex (all 0 for a path of no length).
fn vertex_fractions(path: &[(f64, f64)]) -> Vec<f64> {
    let mut cumulative = vec![0.0];
    for w in path.windows(2) {
        cumulative.push(cumulative.last().unwrap() + metres(w[0], w[1]));
    }
    let total = *cumulative.last().unwrap();
    cumulative
        .into_iter()
        .map(|c| if total > 0.0 { c / total } else { 0.0 })
        .collect()
}

/// The position at `fraction` (0–1) of the path's length.
fn along(path: &[(f64, f64)], fractions: &[f64], fraction: f64) -> (f64, f64) {
    let fraction = fraction.clamp(0.0, 1.0);
    let i = fractions
        .partition_point(|&f| f < fraction)
        .clamp(1, path.len() - 1);
    let (f0, f1) = (fractions[i - 1], fractions[i]);
    let t = if f1 > f0 {
        (fraction - f0) / (f1 - f0)
    } else {
        0.0
    };
    let (a, b) = (path[i - 1], path[i]);
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn time_fraction(time: DateTime<Utc>, from: DateTime<Utc>, to: DateTime<Utc>) -> f64 {
    let span = (to - from).num_milliseconds();
    if span <= 0 {
        0.0
    } else {
        (time - from).num_milliseconds() as f64 / span as f64
    }
}

/// Elevation at every sample: its own, else interpolated in time between the nearest known
/// ones (the nearest one at either end), else 0.
fn filled_elevations(samples: &[RawSample]) -> Vec<f64> {
    let known: Vec<(DateTime<Utc>, f64)> = samples
        .iter()
        .filter_map(|s| Some((s.time, s.elevation?)))
        .collect();
    samples
        .iter()
        .map(|s| {
            if let Some(e) = s.elevation {
                return e;
            }
            let after = known.partition_point(|(t, _)| *t < s.time);
            match (after.checked_sub(1).map(|i| known[i]), known.get(after)) {
                (Some((t0, e0)), Some(&(t1, e1))) => lerp(e0, e1, time_fraction(s.time, t0, t1)),
                (Some((_, e)), None) | (None, Some(&(_, e))) => e,
                (None, None) => 0.0,
            }
        })
        .collect()
}

/// Applies one edit: every sample from `from` to `to` moves onto the drawn path, at the share
/// of the path its device distance has covered within the stretch, else its share of the time.
/// Gaps longer than [`FILL_GAP`] get a sample every second.
fn apply_edit(samples: Vec<RawSample>, edit: &FixEdit) -> AppResult<Vec<RawSample>> {
    if edit.path.len() < 2 {
        return Err(AppError::Invalid("a drawn route needs two points".into()));
    }
    let (Some(from), Some(to)) = (
        DateTime::from_timestamp_millis(edit.from),
        DateTime::from_timestamp_millis(edit.to),
    ) else {
        return Err(AppError::Invalid("invalid time range".into()));
    };
    if to <= from {
        return Err(AppError::Invalid("the stretch to fix is empty".into()));
    }
    let inside = |s: &RawSample| s.time >= from && s.time <= to;
    let in_range: Vec<&RawSample> = samples.iter().filter(|s| inside(s)).collect();
    let device = match (in_range.first(), in_range.last()) {
        (Some(first), Some(last)) if in_range.iter().all(|s| s.distance.is_some()) => {
            let (d0, d1) = (first.distance.unwrap(), last.distance.unwrap());
            (d1 > d0).then_some((d0, d1))
        }
        _ => None,
    };
    let fraction = |s: &RawSample| match (device, s.distance) {
        (Some((d0, d1)), Some(d)) => (d - d0) / (d1 - d0),
        _ => time_fraction(s.time, from, to),
    };

    // Positions and fractions first, so inserted samples can be placed between neighbours.
    let fractions = vertex_fractions(&edit.path);
    let mut out: Vec<RawSample> = Vec::with_capacity(samples.len());
    let mut previous: Option<(DateTime<Utc>, f64)> = None;
    for mut sample in samples {
        if !inside(&sample) {
            previous = None;
            out.push(sample);
            continue;
        }
        let f = fraction(&sample);
        if let Some((t0, f0)) = previous {
            let mut t = t0 + TimeDelta::seconds(1);
            if sample.time - t0 > FILL_GAP {
                while t < sample.time {
                    let share = lerp(f0, f, time_fraction(t, t0, sample.time));
                    out.push(RawSample {
                        time: t,
                        position: Some(along(&edit.path, &fractions, share)),
                        elevation: None,
                        heart_rate: None,
                        cadence: None,
                        power: None,
                        distance: None,
                    });
                    t += TimeDelta::seconds(1);
                }
            }
        }
        sample.position = Some(along(&edit.path, &fractions, f));
        previous = Some((sample.time, f));
        out.push(sample);
    }
    Ok(out)
}

/// The activity with every edit applied. Elevations missing afterwards (for samples that had no
/// GPS, or were inserted) are interpolated, so every sample becomes a point.
pub fn apply(activity: &TcxActivity, edits: &[FixEdit]) -> AppResult<TcxActivity> {
    if edits.is_empty() {
        return Err(AppError::Invalid("nothing to fix".into()));
    }
    let mut samples = activity.samples.clone();
    for edit in edits {
        samples = apply_edit(samples, edit)?;
    }
    let elevations = filled_elevations(&samples);
    for (sample, elevation) in samples.iter_mut().zip(elevations) {
        if sample.position.is_some() {
            sample.elevation = Some(elevation);
        }
    }
    let fixed = TcxActivity::from_samples(activity.sport.clone(), samples);
    if fixed.points.len() < 2 {
        return Err(AppError::Invalid("the fixed track has no route".into()));
    }
    Ok(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(second: i64, position: Option<(f64, f64)>) -> RawSample {
        RawSample {
            time: DateTime::from_timestamp(1_746_522_000 + second, 0).unwrap(),
            position,
            elevation: position.map(|_| 10.0),
            heart_rate: Some(140.0),
            cadence: None,
            power: None,
            distance: None,
        }
    }

    /// A run heading north at about 3 m/s, one sample a second.
    fn straight(seconds: i64) -> Vec<RawSample> {
        (0..=seconds)
            .map(|s| sample(s, Some((48.0 + s as f64 * 0.000027, 2.0))))
            .collect()
    }

    fn run(samples: Vec<RawSample>) -> TcxActivity {
        TcxActivity::from_samples("Running".into(), samples)
    }

    fn length(activity: &TcxActivity) -> f64 {
        activity
            .points
            .windows(2)
            .map(|w| {
                metres(
                    (w[0].latitude, w[0].longitude),
                    (w[1].latitude, w[1].longitude),
                )
            })
            .sum()
    }

    fn millis(activity: &TcxActivity, i: usize) -> i64 {
        activity.samples[i].time.timestamp_millis()
    }

    #[test]
    fn a_clean_run_has_no_issues() {
        assert!(issues(&run(straight(100))).is_empty());
    }

    #[test]
    fn finds_and_fixes_a_spike() {
        let mut samples = straight(100);
        samples[50].position = Some((48.01, 2.0)); // about 1 km off and back
        let activity = run(samples);
        let found = issues(&activity);
        assert_eq!(
            found,
            [FixIssue {
                kind: FixIssueKind::Spike,
                start: 49,
                end: 51
            }]
        );
        let before = length(&activity);
        let a = activity.points[49].clone();
        let b = activity.points[51].clone();
        let fixed = apply(
            &activity,
            &[FixEdit {
                from: millis(&activity, 49),
                to: millis(&activity, 51),
                path: vec![(a.latitude, a.longitude), (b.latitude, b.longitude)],
            }],
        )
        .unwrap();
        assert!(before > 2000.0);
        assert!((length(&fixed) - length(&run(straight(100)))).abs() < 0.5);
        assert!(issues(&fixed).is_empty());
        // Everything else stays.
        assert_eq!(fixed.samples.len(), 101);
        assert_eq!(fixed.samples[50].heart_rate, Some(140.0));
        assert_eq!(fixed.samples[20], activity.samples[20]);
    }

    #[test]
    fn finds_and_fills_a_dropout() {
        let mut samples = straight(100);
        for s in &mut samples[30..60] {
            s.position = None;
            s.elevation = None;
        }
        let activity = run(samples);
        assert_eq!(activity.points.len(), 71);
        let found = issues(&activity);
        assert_eq!(
            found,
            [FixIssue {
                kind: FixIssueKind::Gap,
                start: 29,
                end: 60
            }]
        );
        let (a, b) = (activity.samples[29].clone(), activity.samples[60].clone());
        let fixed = apply(
            &activity,
            &[FixEdit {
                from: millis(&activity, 29),
                to: millis(&activity, 60),
                path: vec![a.position.unwrap(), b.position.unwrap()],
            }],
        )
        .unwrap();
        assert_eq!(fixed.points.len(), 101);
        assert_eq!(fixed.points[45].elevation, 10.0);
        let expected = length(&run(straight(100)));
        assert!((length(&fixed) - expected).abs() < 0.5);
    }

    #[test]
    fn fills_a_stretch_without_samples() {
        let mut samples = straight(10);
        samples.extend((70..=80).map(|s| sample(s, Some((48.01, 2.0)))));
        let activity = run(samples);
        assert_eq!(
            issues(&activity),
            [FixIssue {
                kind: FixIssueKind::Gap,
                start: 10,
                end: 11
            }]
        );
        let fixed = apply(
            &activity,
            &[FixEdit {
                from: millis(&activity, 10),
                to: millis(&activity, 11),
                path: vec![activity.samples[10].position.unwrap(), (48.01, 2.0)],
            }],
        )
        .unwrap();
        // A sample every second from 11 to 69.
        assert_eq!(fixed.samples.len(), 22 + 59);
        assert!(
            fixed
                .samples
                .windows(2)
                .all(|w| w[1].time - w[0].time <= FILL_GAP)
        );
    }

    #[test]
    fn draws_a_route_for_a_run_without_gps() {
        let samples: Vec<RawSample> = (0..=600)
            .map(|s| RawSample {
                distance: Some(s as f64 * 3.0),
                ..sample(s, None)
            })
            .collect();
        let activity = run(samples);
        assert!(activity.points.is_empty());
        assert_eq!(
            issues(&activity),
            [FixIssue {
                kind: FixIssueKind::NoGps,
                start: 0,
                end: 600
            }]
        );
        // An L: 1 km north, then 800 m east.
        let corner = (48.0 + 1000.0 / 111_200.0, 2.0);
        let path = vec![(48.0, 2.0), corner, (corner.0, 2.0 + 800.0 / 74_400.0)];
        let fixed = apply(
            &activity,
            &[FixEdit {
                from: millis(&activity, 0),
                to: millis(&activity, 600),
                path: path.clone(),
            }],
        )
        .unwrap();
        assert_eq!(fixed.points.len(), 601);
        let drawn: f64 = path.windows(2).map(|w| metres(w[0], w[1])).sum();
        assert!(
            (length(&fixed) - drawn).abs() < 2.0,
            "{} vs {drawn}",
            length(&fixed)
        );
        assert_eq!(fixed.points[0].elevation, 0.0);
    }

    #[test]
    fn places_samples_by_device_distance() {
        // Slow first half, fast second half: by distance, the middle sample is a quarter in.
        let samples: Vec<RawSample> = [0.0, 100.0, 400.0]
            .iter()
            .enumerate()
            .map(|(i, d)| RawSample {
                distance: Some(*d),
                ..sample(i as i64, None)
            })
            .collect();
        let activity = run(samples);
        let fixed = apply(
            &activity,
            &[FixEdit {
                from: millis(&activity, 0),
                to: millis(&activity, 2),
                path: vec![(48.0, 2.0), (48.004, 2.0)],
            }],
        )
        .unwrap();
        assert!((fixed.points[1].latitude - 48.001).abs() < 1e-9);
    }

    #[test]
    fn rejects_empty_edits() {
        let activity = run(straight(10));
        assert!(apply(&activity, &[]).is_err());
        let edit = FixEdit {
            from: millis(&activity, 5),
            to: millis(&activity, 2),
            path: vec![(48.0, 2.0), (48.1, 2.0)],
        };
        assert!(apply(&activity, &[edit]).is_err());
    }
}
