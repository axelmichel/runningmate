//! Training effect after the EPOC model of Firstbeat (used by Garmin, Suunto and Polar): the
//! excess post-exercise oxygen consumption (ml/kg) is accumulated second by second from the heart
//! rate, the aerobic training effect (0–5) follows from its peak relative to the fitness level,
//! the anaerobic one from the time spent close to the maximum heart rate. Firstbeat's method is
//! proprietary; this is a documented approximation of its published form
//! (`docs/metrics/trainings-effect.md`).

use serde::Serialize;
use ts_rs::TS;

use super::health::Vo2maxSource;
use crate::importer::track::{PAUSE_THRESHOLD, Track};

/// EPOC generation `GEN_RATE · (e^(GEN_EXPONENT · I) − 1)` in ml/kg/min at intensity `I`
/// (share of the heart-rate reserve).
const GEN_RATE: f64 = 0.015;
const GEN_EXPONENT: f64 = 6.5;
/// Time constant `TAU_REST + TAU_SLOPE · I³` (minutes) with which EPOC approaches the steady
/// state of the intensity: quick at rest, slow when hard.
const TAU_REST: f64 = 10.0;
const TAU_SLOPE: f64 = 50.0;

/// Peak EPOC (ml/kg) at aerobic training effect 1, 2, 3, 4 and 5 for an average fitness level
/// (VO2max [`REFERENCE_VO2MAX`]); in between, the effect is interpolated on a log scale.
const AEROBIC_THRESHOLDS: [f64; 5] = [6.0, 15.0, 35.0, 80.0, 180.0];
/// VO2max (ml/kg/min) the thresholds are set for.
const REFERENCE_VO2MAX: f64 = 45.0;
/// Range of the fitness factor that scales the thresholds.
const FITNESS_SCALE: (f64, f64) = (0.75, 1.4);

/// Intensity above which time counts towards the anaerobic load (upper zone 4).
const ANAEROBIC_FLOOR: f64 = 0.85;
/// Intensity of a burst (zone 5 of the Karvonen zones) …
const BURST_INTENSITY: f64 = 0.9;
/// … held at least this many seconds.
const BURST_SECONDS: f64 = 15.0;
/// Each burst after the first adds this share to the anaerobic load, up to `BURST_BONUS_MAX`.
const BURST_BONUS: f64 = 0.25;
const BURST_BONUS_MAX: f64 = 2.0;
/// Anaerobic load (weighted minutes) at which the anaerobic effect reaches 1 − 1/e of 5.
const ANAEROBIC_SCALE: f64 = 15.0;

/// Recovery `RECOVERY_HOURS · (EPOC / threshold of effect 3)^RECOVERY_EXPONENT`, at most
/// `RECOVERY_MAX_HOURS`.
const RECOVERY_HOURS: f64 = 24.0;
const RECOVERY_EXPONENT: f64 = 0.75;
const RECOVERY_MAX_HOURS: f64 = 96.0;

/// Fallbacks for a missing profile value.
pub const DEFAULT_RESTING_HEART_RATE: i64 = 60;
/// Maximum heart rate without profile value, zones or age: 220 − 35.
pub const DEFAULT_MAX_HEART_RATE: i64 = 185;
/// The reserve between rest and maximum must be at least this wide (bpm).
const MIN_RESERVE: i64 = 20;
/// A maximum heart rate below this is a typo rather than a maximum.
const MIN_MAX_HEART_RATE: i64 = 100;

/// Where the maximum heart rate comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MaxHeartRateSource {
    /// Entered in the user settings.
    Profile,
    /// Upper bound of zone 5.
    Zone5,
    /// 220 − age.
    Age,
    /// [`DEFAULT_MAX_HEART_RATE`].
    Default,
}

/// Where the resting heart rate comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RestingHeartRateSource {
    Profile,
    /// [`DEFAULT_RESTING_HEART_RATE`].
    Default,
}

/// Resting and maximum heart rate the intensity is measured against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartRateBasis {
    pub rest: i64,
    pub rest_source: RestingHeartRateSource,
    pub max: i64,
    pub max_source: MaxHeartRateSource,
}

impl HeartRateBasis {
    /// The profile's maximum, else the zone 5 bound, else 220 − age, else 185; the profile's
    /// resting heart rate, else 60. Values of 0 count as missing.
    pub fn resolve(
        hr_max: Option<i64>,
        zone5: Option<i64>,
        age: Option<i64>,
        hr_min: Option<i64>,
    ) -> Self {
        let plausible = |m: &i64| *m >= MIN_MAX_HEART_RATE;
        let (max, max_source) = if let Some(max) = hr_max.filter(plausible) {
            (max, MaxHeartRateSource::Profile)
        } else if let Some(max) = zone5.filter(plausible) {
            (max, MaxHeartRateSource::Zone5)
        } else if let Some(max) = age.filter(|a| *a > 0).map(|a| 220 - a).filter(plausible) {
            (max, MaxHeartRateSource::Age)
        } else {
            (DEFAULT_MAX_HEART_RATE, MaxHeartRateSource::Default)
        };
        let (rest, rest_source) = match hr_min.filter(|r| *r > 0 && *r <= max - MIN_RESERVE) {
            Some(rest) => (rest, RestingHeartRateSource::Profile),
            None => (
                DEFAULT_RESTING_HEART_RATE.min(max - MIN_RESERVE),
                RestingHeartRateSource::Default,
            ),
        };
        Self {
            rest,
            rest_source,
            max,
            max_source,
        }
    }

    /// Share of the heart-rate reserve, 0–1.
    pub fn intensity(&self, heart_rate: f64) -> f64 {
        ((heart_rate - self.rest as f64) / (self.max - self.rest) as f64).clamp(0.0, 1.0)
    }
}

/// A stretch of constant heart rate; `None` for a pause or a stretch without heart rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub heart_rate: Option<f64>,
    pub seconds: f64,
}

/// Samples of a track: each point's heart rate for the time since the previous point. Gaps
/// longer than the importer's pause threshold are pauses.
pub fn track_samples(track: &Track) -> Vec<Sample> {
    track
        .points
        .iter()
        .zip(&track.raw_time_diff)
        .skip(1)
        .filter(|(_, gap)| **gap > 0.0)
        .map(|(point, gap)| Sample {
            heart_rate: point
                .heart_rate
                .filter(|hr| *hr > 0.0 && *gap <= PAUSE_THRESHOLD),
            seconds: *gap,
        })
        .collect()
}

/// Samples of the splits, `(average heart rate, seconds)`, when there is no track.
pub fn split_samples(splits: &[(f64, f64)]) -> Vec<Sample> {
    splits
        .iter()
        .filter(|(_, seconds)| *seconds > 0.0)
        .map(|(hr, seconds)| Sample {
            heart_rate: Some(*hr).filter(|hr| *hr > 0.0),
            seconds: *seconds,
        })
        .collect()
}

/// EPOC generation (ml/kg/min) at intensity `i`.
fn generation(i: f64) -> f64 {
    GEN_RATE * ((GEN_EXPONENT * i).exp() - 1.0)
}

/// Time constant (minutes) at intensity `i`.
fn time_constant(i: f64) -> f64 {
    TAU_REST + TAU_SLOPE * i.powi(3)
}

/// EPOC after `seconds` at intensity `i`, starting from `epoc`: it moves exponentially towards
/// the steady state `generation · τ` of the intensity, up when above, down (recovery) when below.
pub fn epoc_step(epoc: f64, i: f64, seconds: f64) -> f64 {
    let tau = time_constant(i);
    let steady = generation(i) * tau;
    steady + (epoc - steady) * (-seconds / 60.0 / tau).exp()
}

/// What the heart rate of an activity adds up to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accumulated {
    /// ml/kg.
    pub peak_epoc: f64,
    /// Minutes above [`ANAEROBIC_FLOOR`], weighted by the square of how far above.
    pub anaerobic_load: f64,
    /// Stretches of at least [`BURST_SECONDS`] above [`BURST_INTENSITY`].
    pub bursts: u32,
}

/// Accumulates the samples; `None` without any heart rate. Pauses only let EPOC decay as at
/// rest, so they never add any.
pub fn accumulate(samples: &[Sample], basis: &HeartRateBasis) -> Option<Accumulated> {
    if samples.iter().all(|s| s.heart_rate.is_none()) {
        return None;
    }
    let mut epoc: f64 = 0.0;
    let mut peak: f64 = 0.0;
    let mut load = 0.0;
    let mut bursts = 0;
    let mut burst = 0.0;
    for sample in samples {
        let i = sample.heart_rate.map_or(0.0, |hr| basis.intensity(hr));
        // Within a sample the intensity is constant and EPOC moves monotonically, so the peak
        // is at one of its ends.
        epoc = epoc_step(epoc, i, sample.seconds);
        peak = peak.max(epoc);
        if i > ANAEROBIC_FLOOR {
            load +=
                ((i - ANAEROBIC_FLOOR) / (1.0 - ANAEROBIC_FLOOR)).powi(2) * sample.seconds / 60.0;
        }
        if sample.heart_rate.is_some() && i >= BURST_INTENSITY {
            if burst < BURST_SECONDS && burst + sample.seconds >= BURST_SECONDS {
                bursts += 1;
            }
            burst += sample.seconds;
        } else {
            burst = 0.0;
        }
    }
    Some(Accumulated {
        peak_epoc: peak,
        anaerobic_load: load,
        bursts,
    })
}

/// Factor for the thresholds: fitter athletes need more EPOC for the same effect. 1 without a
/// VO2max.
pub fn fitness_scale(vo2max: Option<f64>) -> f64 {
    vo2max.filter(|v| *v > 0.0).map_or(1.0, |v| {
        (v / REFERENCE_VO2MAX).clamp(FITNESS_SCALE.0, FITNESS_SCALE.1)
    })
}

/// Aerobic training effect 0–5 from the peak EPOC: linear up to the threshold of 1, then
/// log-interpolated between the thresholds, 5 at most.
pub fn aerobic_effect(peak_epoc: f64, scale: f64) -> f64 {
    let thresholds = AEROBIC_THRESHOLDS.map(|t| t * scale);
    if peak_epoc <= thresholds[0] {
        return (peak_epoc / thresholds[0]).max(0.0);
    }
    for (i, pair) in thresholds.windows(2).enumerate() {
        if peak_epoc < pair[1] {
            return 1.0 + i as f64 + (peak_epoc / pair[0]).ln() / (pair[1] / pair[0]).ln();
        }
    }
    5.0
}

/// Anaerobic training effect 0–5: `5 · (1 − e^(−load · bonus / 15))` with a bonus of 25 % per
/// repeated burst (at most +200 %).
pub fn anaerobic_effect(load: f64, bursts: u32) -> f64 {
    let bonus = 1.0 + (BURST_BONUS * f64::from(bursts.saturating_sub(1))).min(BURST_BONUS_MAX);
    5.0 * (1.0 - (-load * bonus / ANAEROBIC_SCALE).exp())
}

/// Hours until recovered: 24 at the EPOC of effect 3, at most 96.
pub fn recovery_hours(peak_epoc: f64, scale: f64) -> f64 {
    (RECOVERY_HOURS * (peak_epoc / (AEROBIC_THRESHOLDS[2] * scale)).powf(RECOVERY_EXPONENT))
        .min(RECOVERY_MAX_HOURS)
}

/// The usual wording of a training effect, also for the AI summary.
pub fn label(effect: f64) -> &'static str {
    match round1(effect) {
        e if e < 1.0 => "no effect",
        e if e < 2.0 => "minor",
        e if e < 3.0 => "maintaining",
        e if e < 4.0 => "improving",
        e if e < 5.0 => "highly improving",
        _ => "overreaching",
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrainingEffect {
    /// Aerobic training effect, 0.0–5.0 in steps of 0.1.
    pub aerobic: f64,
    /// Anaerobic training effect, 0.0–5.0 in steps of 0.1.
    pub anaerobic: f64,
    /// Peak EPOC, ml/kg.
    pub peak_epoc: f64,
    /// Suggested rest in hours.
    pub recovery_hours: f64,
    /// The heart rates the intensity is measured against (bpm), and where they come from.
    #[ts(type = "number")]
    pub max_heart_rate: i64,
    pub max_heart_rate_source: MaxHeartRateSource,
    #[ts(type = "number")]
    pub resting_heart_rate: i64,
    pub resting_heart_rate_source: RestingHeartRateSource,
    /// VO2max (ml/kg/min) the thresholds are scaled with; `None` for an average fitness level.
    pub vo2max: Option<f64>,
    pub vo2max_source: Option<Vo2maxSource>,
    /// Computed from the split averages because the activity has no track.
    pub from_splits: bool,
}

/// The training effect of the samples; `None` without any heart rate.
pub fn evaluate(
    samples: &[Sample],
    basis: &HeartRateBasis,
    vo2max: Option<(f64, Vo2maxSource)>,
    from_splits: bool,
) -> Option<TrainingEffect> {
    let total = accumulate(samples, basis)?;
    let scale = fitness_scale(vo2max.map(|(v, _)| v));
    Some(TrainingEffect {
        aerobic: round1(aerobic_effect(total.peak_epoc, scale)),
        anaerobic: round1(anaerobic_effect(total.anaerobic_load, total.bursts)),
        peak_epoc: round1(total.peak_epoc),
        recovery_hours: round1(recovery_hours(total.peak_epoc, scale)),
        max_heart_rate: basis.max,
        max_heart_rate_source: basis.max_source,
        resting_heart_rate: basis.rest,
        resting_heart_rate_source: basis.rest_source,
        vo2max: vo2max.map(|(v, _)| round1(v)),
        vo2max_source: vo2max.map(|(_, s)| s),
        from_splits,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rest 60, max 190: a reserve of 130 bpm.
    fn basis() -> HeartRateBasis {
        HeartRateBasis::resolve(Some(190), None, None, Some(60))
    }

    /// Heart rate at a share of the reserve of [`basis`].
    fn bpm(share: f64) -> f64 {
        60.0 + 130.0 * share
    }

    /// One sample per second for `minutes` at each `(share, minutes)`.
    fn series(parts: &[(f64, f64)]) -> Vec<Sample> {
        parts
            .iter()
            .flat_map(|(share, minutes)| {
                (0..(minutes * 60.0) as usize).map(|_| Sample {
                    heart_rate: Some(bpm(*share)),
                    seconds: 1.0,
                })
            })
            .collect()
    }

    fn effect(samples: &[Sample]) -> TrainingEffect {
        evaluate(samples, &basis(), None, false).unwrap()
    }

    #[test]
    fn easy_steady_run_maintains() {
        let easy = effect(&series(&[(0.65, 60.0)]));
        assert!((2.0..3.0).contains(&easy.aerobic), "{easy:?}");
        assert_eq!(easy.anaerobic, 0.0);
        assert!((10.0..24.0).contains(&easy.recovery_hours), "{easy:?}");
        // Shorter or easier is less.
        assert!(effect(&series(&[(0.65, 30.0)])).aerobic < easy.aerobic);
        assert!(effect(&series(&[(0.35, 60.0)])).aerobic < 1.0);
    }

    #[test]
    fn threshold_intervals_improve_both() {
        let mut parts = vec![(0.6, 15.0)];
        for _ in 0..5 {
            parts.extend([(0.93, 5.0), (0.55, 3.0)]);
        }
        parts.push((0.6, 10.0));
        let intervals = effect(&series(&parts));
        let easy = effect(&series(&[(0.65, 60.0)]));
        assert!(intervals.aerobic >= 3.5, "{intervals:?}");
        assert!(intervals.anaerobic >= 2.0, "{intervals:?}");
        assert!(intervals.peak_epoc > 3.0 * easy.peak_epoc);
        assert!(intervals.recovery_hours > easy.recovery_hours);
        assert_eq!(accumulate(&series(&parts), &basis()).unwrap().bursts, 5);
        // The same time near the maximum in one piece earns less anaerobic effect.
        let steady = effect(&series(&[(0.6, 15.0), (0.93, 25.0), (0.6, 10.0)]));
        assert!(steady.anaerobic < intervals.anaerobic);
    }

    #[test]
    fn effect_is_capped_and_labelled() {
        let race = effect(&series(&[(0.97, 90.0)]));
        assert_eq!(race.aerobic, 5.0);
        assert!(race.anaerobic <= 5.0);
        assert_eq!(race.recovery_hours, 96.0);
        assert_eq!(label(0.94), "no effect");
        assert_eq!(label(0.96), "minor");
        assert_eq!(label(2.0), "maintaining");
        assert_eq!(label(4.9), "highly improving");
        assert_eq!(label(5.0), "overreaching");
        assert_eq!(aerobic_effect(0.0, 1.0), 0.0);
        assert!((aerobic_effect(35.0, 1.0) - 3.0).abs() < 1e-9);
        assert!((aerobic_effect(70.0, 2.0) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn fitter_athletes_need_more_for_the_same_effect() {
        let samples = series(&[(0.8, 40.0)]);
        let average = evaluate(&samples, &basis(), None, false).unwrap();
        let fit = evaluate(&samples, &basis(), Some((60.0, Vo2maxSource::Runs)), false).unwrap();
        assert_eq!(fit.peak_epoc, average.peak_epoc);
        assert!(fit.aerobic < average.aerobic);
        assert!(fit.recovery_hours < average.recovery_hours);
        assert_eq!(fit.vo2max_source, Some(Vo2maxSource::Runs));
    }

    #[test]
    fn pauses_add_nothing() {
        let run = series(&[(0.8, 30.0)]);
        let plain = accumulate(&run, &basis()).unwrap();
        // A 20-minute pause recorded as one gap with a high heart rate at the restart.
        let mut paused = run.clone();
        paused.push(Sample {
            heart_rate: None,
            seconds: 1200.0,
        });
        let after = accumulate(&paused, &basis()).unwrap();
        assert_eq!(after.peak_epoc, plain.peak_epoc);
        assert_eq!(after.anaerobic_load, plain.anaerobic_load);

        // The track marks gaps over 10 s as pauses, whatever the heart rate after them.
        let split = effect(
            &[
                series(&[(0.8, 15.0)]),
                vec![Sample {
                    heart_rate: None,
                    seconds: 600.0,
                }],
                series(&[(0.8, 15.0)]),
            ]
            .concat(),
        );
        assert!(split.peak_epoc < effect(&run).peak_epoc);
        assert!(
            accumulate(
                &[Sample {
                    heart_rate: None,
                    seconds: 60.0
                }],
                &basis()
            )
            .is_none()
        );
    }

    #[test]
    fn splits_give_a_similar_result() {
        let from_track = effect(&series(&[(0.75, 45.0)]));
        let from_splits = evaluate(
            &split_samples(&[(bpm(0.75), 900.0), (bpm(0.75), 900.0), (bpm(0.75), 900.0)]),
            &basis(),
            None,
            true,
        )
        .unwrap();
        assert!((from_splits.peak_epoc - from_track.peak_epoc).abs() < 0.5);
        assert!(from_splits.from_splits);
    }

    #[test]
    fn falls_back_without_rest_or_max() {
        let profile = basis();
        assert_eq!(profile.max_source, MaxHeartRateSource::Profile);
        assert_eq!(profile.rest_source, RestingHeartRateSource::Profile);

        let zone5 = HeartRateBasis::resolve(None, Some(178), Some(50), None);
        assert_eq!(
            (zone5.max, zone5.max_source),
            (178, MaxHeartRateSource::Zone5)
        );
        assert_eq!(
            (zone5.rest, zone5.rest_source),
            (60, RestingHeartRateSource::Default)
        );

        let age = HeartRateBasis::resolve(Some(0), None, Some(50), Some(0));
        assert_eq!((age.max, age.max_source), (170, MaxHeartRateSource::Age));

        let nothing = HeartRateBasis::resolve(None, None, None, None);
        assert_eq!(
            (nothing.max, nothing.max_source),
            (185, MaxHeartRateSource::Default)
        );
        assert_eq!(nothing.rest, 60);

        // A resting heart rate too close to the maximum is not used.
        let close = HeartRateBasis::resolve(Some(150), None, None, Some(140));
        assert_eq!(close.rest_source, RestingHeartRateSource::Default);
        assert_eq!(close.intensity(60.0), 0.0);
        assert_eq!(close.intensity(200.0), 1.0);
    }
}
