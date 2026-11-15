//! Builds a running plan from fixed rules, after the usual plans (Daniels, Pfitzinger, Higdon):
//! phases from base to taper, a weekly volume growing by at most 10 % with an easier week every
//! fourth week, a long run, and one or two hard sessions a week that never follow each other.
//! A pure function of the baseline and the input, so the wizard can show it before saving.

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{
    baseline::Baseline,
    paces::{self, PaceRange, Paces},
};
use crate::{
    analysis::cardio,
    error::{AppError, AppResult},
};

/// km per week a plan starts with at least.
const MIN_WEEK_KM: f64 = 10.0;
/// Weekly growth of the volume.
const GROWTH: f64 = 1.10;
/// Volume of an easier week, relative to the week before.
const EASIER: f64 = 0.75;
/// How much longer the long run may get from one week to the next, km.
const LONG_STEP: f64 = 2.0;
/// Shortest easy run, km.
const MIN_EASY_KM: f64 = 3.0;
/// Warm-up and cool-down around the work of a hard session, km, at most and at least.
const AROUND_KM: f64 = 3.5;
const MIN_AROUND_KM: f64 = 2.0;
/// Share of the week's km the work of one hard session may take, after Daniels' limits.
const WORK_SHARE: f64 = 0.2;
/// Longest plan, a year.
pub const MAX_WEEKS: u32 = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TrainingTarget {
    /// A race over `distance`.
    Race,
    /// Getting in shape: mostly easy running, one faster run a week.
    Shape,
    /// More fitness: intervals and tempo runs.
    Fitness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Phase {
    Base,
    Build,
    Peak,
    Taper,
    /// An easier week of a shape or fitness plan.
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SessionKind {
    Easy,
    Recovery,
    Long,
    Tempo,
    Intervals,
    Race,
}

/// The work of a tempo or interval session: `reps` times `metres` or `seconds`, with `rest`
/// seconds of jogging in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Structure {
    pub reps: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub metres: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub seconds: Option<u32>,
    pub rest: u32,
}

impl Structure {
    /// km of work, at `pace` s/km for timed reps.
    pub fn work_km(&self, pace: Option<f64>) -> f64 {
        let each = match (self.metres, self.seconds) {
            (Some(metres), _) => f64::from(metres) / 1000.0,
            (None, Some(seconds)) => f64::from(seconds) / pace.unwrap_or(330.0),
            (None, None) => 0.0,
        };
        f64::from(self.reps) * each
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanInput {
    pub name: String,
    pub target: TrainingTarget,
    /// km, races only.
    pub distance: Option<f64>,
    /// s, the time aimed for in the race.
    pub target_time: Option<u32>,
    /// "YYYY-MM-DD", the first day of the plan.
    pub start: String,
    pub weeks: u32,
    /// Training weekdays, 0 = Monday.
    pub days: Vec<u8>,
    /// Weekday of the long run and the race.
    pub long_day: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionDraft {
    /// "YYYY-MM-DD".
    pub date: String,
    /// 0-based week of the plan.
    pub week: u32,
    pub phase: Phase,
    pub kind: SessionKind,
    /// km, warm-up and cool-down included.
    pub distance: f64,
    /// Pace of the run, or of the work of a tempo or interval session.
    pub pace: Option<PaceRange>,
    pub structure: Option<Structure>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "code", rename_all = "camelCase")]
#[ts(export)]
pub enum PlanWarning {
    /// Fewer weeks than `weeks` are usual for the distance.
    TooShort { weeks: u32 },
    /// The recent volume is far below the `km` per week the distance needs.
    LowVolume { km: f64 },
    /// No runs or profile value to take paces from; sessions only have distances.
    NoPaces,
    /// The target time needs a VDOT of `vdot`, well above the current one.
    Ambitious { vdot: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Preview {
    pub sessions: Vec<SessionDraft>,
    pub warnings: Vec<PlanWarning>,
    /// "YYYY-MM-DD", the race day or the last day of the plan.
    pub end: String,
    pub paces: Option<Paces>,
    /// s/km aimed for in the race.
    pub race_pace: Option<f64>,
}

/// Where "Adapt plan" picks up: the week holding today, with the volume to go on from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resume {
    pub week: u32,
    pub km: f64,
    pub longest_km: f64,
}

/// Rules that depend on the race distance, for 5k, 10k, half and marathon.
struct DistanceRules {
    peak_km: f64,
    long_cap: f64,
    min_weeks: u32,
    needed_km: f64,
    taper_weeks: u32,
    /// Share of the week's km the long run gets, with four or more training days.
    long_share: f64,
    /// The long and middle-distance races train at marathon pace rather than intervals.
    endurance: bool,
}

fn rules(target: TrainingTarget, distance: f64) -> DistanceRules {
    let rule = |peak_km, long_cap, min_weeks, needed_km, taper_weeks, long_share, endurance| {
        DistanceRules {
            peak_km,
            long_cap,
            min_weeks,
            needed_km,
            taper_weeks,
            long_share,
            endurance,
        }
    };
    match target {
        TrainingTarget::Shape => rule(f64::INFINITY, 16.0, 0, 0.0, 0, 0.30, false),
        TrainingTarget::Fitness => rule(f64::INFINITY, 18.0, 0, 0.0, 0, 0.30, false),
        TrainingTarget::Race if distance <= 6.0 => rule(35.0, 14.0, 4, 10.0, 1, 0.30, false),
        TrainingTarget::Race if distance <= 12.0 => rule(45.0, 17.0, 6, 15.0, 1, 0.30, false),
        TrainingTarget::Race if distance <= 25.0 => rule(55.0, 21.0, 8, 20.0, 2, 0.35, true),
        TrainingTarget::Race => rule(70.0, 32.0, 12, 30.0, 2, 0.42, true),
    }
}

pub fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::Invalid(format!("date must be YYYY-MM-DD: {value}")))
}

pub fn format_date(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

pub fn weekday(day: NaiveDate) -> u8 {
    day.weekday().num_days_from_monday() as u8
}

/// Checks the input and returns its start date.
pub fn validate(input: &PlanInput) -> AppResult<NaiveDate> {
    let invalid = |message: &str| Err(AppError::Invalid(message.into()));
    if input.name.trim().is_empty() {
        return invalid("a plan needs a name");
    }
    if !(1..=MAX_WEEKS).contains(&input.weeks) {
        return invalid("a plan lasts between 1 and 52 weeks");
    }
    if input.days.is_empty() || input.days.iter().any(|d| *d > 6) {
        return invalid("pick at least one training day");
    }
    if !input.days.contains(&input.long_day) {
        return invalid("the long run must be on a training day");
    }
    match input.target {
        TrainingTarget::Race => {
            if !input
                .distance
                .is_some_and(|d| d.is_finite() && (1.0..=100.0).contains(&d))
            {
                return invalid("the race distance must be between 1 and 100 km");
            }
        }
        _ if input.distance.is_some() || input.target_time.is_some() => {
            return invalid("only a race has a distance and a target time");
        }
        _ => {}
    }
    if input.target_time == Some(0) {
        return invalid("the target time must be above zero");
    }
    parse_date(&input.start)
}

/// The phase of every week.
fn phases(input: &PlanInput, rules: &DistanceRules) -> Vec<Phase> {
    let weeks = input.weeks;
    if input.target != TrainingTarget::Race {
        return (0..weeks)
            .map(|i| {
                if (i + 1) % 4 == 0 {
                    Phase::Recovery
                } else {
                    Phase::Build
                }
            })
            .collect();
    }
    let taper = rules.taper_weeks.min(weeks.saturating_sub(1)).max(1);
    let rest = weeks - taper;
    let peak = (f64::from(rest) * 0.2).round() as u32;
    let build = (f64::from(rest) * 0.4).round() as u32;
    let base = rest - peak - build;
    [
        (Phase::Base, base),
        (Phase::Build, build),
        (Phase::Peak, peak),
        (Phase::Taper, taper),
    ]
    .into_iter()
    .flat_map(|(phase, n)| std::iter::repeat_n(phase, n as usize))
    .collect()
}

/// The planned km of every week; `None` for weeks before `resume`.
fn volumes(
    phases: &[Phase],
    target: TrainingTarget,
    start_km: f64,
    peak_km: f64,
    resume: Option<Resume>,
) -> Vec<Option<f64>> {
    let first = resume.map_or(0, |r| r.week as usize);
    let taper_from = phases
        .iter()
        .position(|p| *p == Phase::Taper)
        .unwrap_or(phases.len());
    let taper_weeks = phases.len() - taper_from;
    let mut level = resume.map_or(start_km, |r| r.km);
    phases
        .iter()
        .enumerate()
        .map(|(i, phase)| {
            if i < first {
                return None;
            }
            let grows = i > first;
            Some(match phase {
                Phase::Taper => {
                    // Two taper weeks go down to 75 % and then to 55 %, one to 60 %.
                    let left = phases.len() - i;
                    match (taper_weeks, left) {
                        (1, _) => level * 0.6,
                        (_, 1) => level * 0.55,
                        _ => level * EASIER,
                    }
                }
                Phase::Recovery => level * EASIER,
                // Every fourth week of a race plan is easier, up to the taper.
                _ if target == TrainingTarget::Race && (i + 1) % 4 == 0 && i + 1 < taper_from => {
                    level * EASIER
                }
                _ => {
                    if grows {
                        level = (level * GROWTH).min(peak_km.max(level));
                    }
                    level
                }
            })
        })
        .collect()
}

/// Shortest distance between two weekdays, counting over the weekend.
fn apart(a: u8, b: u8) -> u8 {
    let d = a.abs_diff(b);
    d.min(7 - d)
}

/// Up to `count` weekdays for hard sessions, none of them next to the long run or to each other.
fn hard_days(days: &[u8], long_day: u8, count: usize) -> Vec<u8> {
    let mut chosen = Vec::new();
    while chosen.len() < count {
        let taken = |d: u8| d == long_day || chosen.contains(&d);
        // The day farthest from the long run and the other hard days, the earliest on a tie.
        let best = days
            .iter()
            .copied()
            .filter(|d| !taken(*d))
            .map(|d| {
                let gap = std::iter::once(long_day)
                    .chain(chosen.iter().copied())
                    .map(|other| apart(d, other))
                    .min()
                    .unwrap_or(7);
                (gap, d)
            })
            .filter(|(gap, _)| *gap >= 2)
            .max_by_key(|(gap, d)| (*gap, std::cmp::Reverse(*d)));
        match best {
            Some((_, day)) => chosen.push(day),
            None => break,
        }
    }
    chosen.sort_unstable();
    chosen
}

/// Hard sessions in a week of `phase`.
fn hard_count(target: TrainingTarget, phase: Phase, days: usize) -> usize {
    let by_days = match days {
        0 | 1 => 0,
        2 | 3 => 1,
        _ => 2,
    };
    match (target, phase) {
        (_, Phase::Recovery) => 0,
        (TrainingTarget::Shape, _) | (_, Phase::Base | Phase::Taper) => by_days.min(1),
        _ => by_days,
    }
}

fn lerp(from: f64, to: f64, progress: f64) -> u32 {
    (from + (to - from) * progress).round() as u32
}

/// A tempo or interval session: kind, structure and pace of the work.
fn hard_session(
    target: TrainingTarget,
    rules: &DistanceRules,
    phase: Phase,
    slot: usize,
    progress: f64,
    paces: Option<&Paces>,
    race_pace: Option<PaceRange>,
) -> (SessionKind, Structure, Option<PaceRange>) {
    let threshold = paces.map(|p| p.threshold);
    let interval = paces.map(|p| p.interval);
    let marathon = paces.map(|p| p.marathon);
    let timed = |reps, minutes, rest| Structure {
        reps,
        metres: None,
        seconds: Some(minutes * 60),
        rest,
    };
    let reps = |reps, metres, rest| Structure {
        reps,
        metres: Some(metres),
        seconds: None,
        rest,
    };
    let tempo = SessionKind::Tempo;
    let intervals = SessionKind::Intervals;
    match (target, phase, slot) {
        (TrainingTarget::Shape, ..) => (tempo, timed(lerp(3.0, 5.0, progress), 4, 120), threshold),
        (TrainingTarget::Fitness, _, 0) => (
            intervals,
            reps(lerp(5.0, 8.0, progress), 800, 120),
            interval,
        ),
        (TrainingTarget::Fitness, ..) => {
            (tempo, timed(lerp(2.0, 3.0, progress), 10, 120), threshold)
        }
        (_, Phase::Base, _) => (
            tempo,
            timed(lerp(2.0, 3.0, progress), lerp(8.0, 10.0, progress), 120),
            threshold,
        ),
        (_, Phase::Build, 0) => (tempo, timed(lerp(3.0, 4.0, progress), 10, 120), threshold),
        (_, Phase::Build, _) if rules.endurance => {
            (tempo, reps(lerp(2.0, 3.0, progress), 3000, 120), marathon)
        }
        (_, Phase::Build, _) => (
            intervals,
            reps(lerp(5.0, 6.0, progress), 1000, 180),
            interval,
        ),
        (_, Phase::Peak, 0) if rules.endurance => (
            tempo,
            reps(lerp(2.0, 3.0, progress), 4000, 120),
            race_pace.or(marathon),
        ),
        (_, Phase::Peak, 0) => (
            intervals,
            reps(lerp(4.0, 5.0, progress), 1600, 180),
            race_pace.or(interval),
        ),
        (_, Phase::Peak, _) => (tempo, timed(1, 20, 0), threshold),
        (_, _, _) if rules.endurance => (intervals, reps(3, 2000, 180), race_pace.or(threshold)),
        (_, _, _) => (intervals, reps(4, 1000, 180), race_pace.or(interval)),
    }
}

/// `structure` with fewer or shorter reps, so its work stays within `max_km`.
fn fit(structure: Structure, pace: Option<f64>, max_km: f64) -> Structure {
    let work = structure.work_km(pace);
    if work <= max_km {
        return structure;
    }
    let reps = ((f64::from(structure.reps) * max_km / work).floor() as u32).max(1);
    let fitted = Structure { reps, ..structure };
    match fitted.seconds {
        // A single timed rep can still be shortened, to 5 minutes at the least.
        Some(seconds) if fitted.work_km(pace) > max_km => {
            let scale = max_km / fitted.work_km(pace);
            Structure {
                seconds: Some(((f64::from(seconds) * scale / 60.0).floor() as u32).max(5) * 60),
                ..fitted
            }
        }
        _ => fitted,
    }
}

fn round_half(km: f64) -> f64 {
    (km * 2.0).round() / 2.0
}

fn mid(range: Option<PaceRange>) -> Option<f64> {
    range.map(|r| (r.fast + r.slow) / 2.0)
}

/// The plan for `input`, starting from `baseline`; from the week of `resume` on when given.
pub fn generate(
    baseline: &Baseline,
    input: &PlanInput,
    resume: Option<Resume>,
) -> AppResult<Preview> {
    let start = validate(input)?;
    let distance = input.distance.unwrap_or(0.0);
    let rules = rules(input.target, distance);
    let mut warnings = Vec::new();

    if input.target == TrainingTarget::Race && input.weeks < rules.min_weeks {
        warnings.push(PlanWarning::TooShort {
            weeks: rules.min_weeks,
        });
    }
    let start_km = baseline
        .weekly_km
        .max(baseline.weekly_km_long * 0.8)
        .max(MIN_WEEK_KM);
    if baseline.weekly_km < rules.needed_km * 0.5 {
        warnings.push(PlanWarning::LowVolume {
            km: rules.needed_km,
        });
    }

    // Paces from the current VDOT; a target time sets the race pace, and when it needs more
    // than the current VDOT training still starts from where the runner is.
    let target_vdot = input
        .target_time
        .zip(input.distance)
        .and_then(|(time, km)| cardio::vdot(km * 1000.0, f64::from(time)));
    let vdot = match (baseline.vdot, target_vdot) {
        (Some(current), Some(target)) => {
            if target > current + 5.0 {
                warnings.push(PlanWarning::Ambitious { vdot: target });
            }
            Some(current.min(target))
        }
        (current, target) => current.or(target),
    };
    let paces = vdot.map(paces::paces);
    if paces.is_none() {
        warnings.push(PlanWarning::NoPaces);
    }
    let race_pace = match (input.target_time, input.distance, vdot) {
        (Some(time), Some(km), _) => Some(f64::from(time) / km),
        (None, Some(km), Some(vdot)) => Some(paces::race_time(vdot, km) / km),
        _ => None,
    }
    .filter(|_| input.target == TrainingTarget::Race);
    let race_range = race_pace.map(|pace| PaceRange::around(pace, 3.0));
    let easy = paces.map(|p| p.easy);
    let recovery = easy.map(|e| PaceRange {
        fast: e.slow - 10.0,
        slow: e.slow + 20.0,
    });

    let phases = phases(input, &rules);
    let peak_km = rules.peak_km.min(
        start_km
            * if input.target == TrainingTarget::Shape {
                1.3
            } else {
                1.5
            },
    );
    let volumes = volumes(&phases, input.target, start_km, peak_km, resume);
    let mut days = input.days.clone();
    days.sort_unstable();
    days.dedup();

    let mut sessions = Vec::new();
    let mut longest = resume
        .map_or(baseline.longest_km, |r| r.longest_km)
        .max(5.0);
    let last_week = input.weeks - 1;
    for (week, (phase, volume)) in phases.iter().zip(&volumes).enumerate() {
        let Some(volume) = *volume else { continue };
        let week = week as u32;
        let first = start + Days::new(u64::from(week) * 7);
        let dates: Vec<NaiveDate> = (0..7)
            .map(|d| first + Days::new(d))
            .filter(|day| days.contains(&weekday(*day)))
            .collect();
        let progress = {
            let same: Vec<usize> = (0..phases.len()).filter(|i| phases[*i] == *phase).collect();
            let at = same.iter().position(|i| *i == week as usize).unwrap_or(0);
            if same.len() > 1 {
                at as f64 / (same.len() - 1) as f64
            } else {
                0.5
            }
        };
        let draft = |day: NaiveDate, kind, distance: f64, pace, structure| SessionDraft {
            date: format_date(day),
            week,
            phase: *phase,
            kind,
            distance: round_half(distance).max(1.0),
            pace,
            structure,
        };

        // Race week: easy runs before the race, nothing after it.
        if input.target == TrainingTarget::Race && week == last_week {
            let race_day = dates
                .iter()
                .copied()
                .find(|d| weekday(*d) == input.long_day)
                .unwrap_or(first + Days::new(6));
            let before: Vec<NaiveDate> = dates.iter().copied().filter(|d| *d < race_day).collect();
            let shakeout =
                ((volume - distance) / before.len().max(1) as f64).clamp(MIN_EASY_KM, 6.0);
            // At most three runs before the race, the ones closest to it but not the day before.
            let picked: Vec<NaiveDate> = before
                .iter()
                .rev()
                .copied()
                .filter(|d| (race_day - *d).num_days() >= 2 || before.len() == 1)
                .take(3)
                .collect();
            for day in picked.into_iter().rev() {
                sessions.push(draft(day, SessionKind::Recovery, shakeout, recovery, None));
            }
            sessions.push(SessionDraft {
                distance,
                ..draft(race_day, SessionKind::Race, distance, race_range, None)
            });
            continue;
        }

        let hard = hard_days(
            &days,
            input.long_day,
            hard_count(input.target, *phase, days.len()),
        );
        let has_long = days.len() > 1 || hard.is_empty();
        let share = rules.long_share + if days.len() <= 3 { 0.05 } else { 0.0 };
        let long_km = if has_long {
            (volume * share)
                .min(rules.long_cap)
                .min(longest + LONG_STEP)
                .max(MIN_EASY_KM)
        } else {
            0.0
        };
        longest = longest.max(long_km);

        let mut week_sessions: Vec<SessionDraft> = Vec::new();
        let mut planned = long_km;
        for (slot, day) in hard.iter().enumerate() {
            let Some(date) = dates.iter().copied().find(|d| weekday(*d) == *day) else {
                continue;
            };
            let (kind, structure, pace) = hard_session(
                input.target,
                &rules,
                *phase,
                slot,
                progress,
                paces.as_ref(),
                race_range,
            );
            // Hard sessions stay a small part of the week and shorter than the long run.
            let around = (volume * 0.25).clamp(MIN_AROUND_KM, AROUND_KM);
            let max_work = (volume * WORK_SHARE).min(long_km - around).max(1.0);
            let structure = fit(structure, mid(pace), max_work);
            let km = around + structure.work_km(mid(pace));
            planned += km;
            week_sessions.push(draft(date, kind, km, pace, Some(structure)));
        }
        let mut easy_days: Vec<NaiveDate> = dates
            .iter()
            .copied()
            .filter(|d| weekday(*d) != input.long_day && !hard.contains(&weekday(*d)))
            .collect();
        // Fewer easy runs when the rest of the week is too little for all of them.
        let left = volume - planned;
        while !easy_days.is_empty() && left / (easy_days.len() as f64) < MIN_EASY_KM {
            // The run right after the long run goes last; it is the one to keep.
            let drop = easy_days
                .iter()
                .position(|d| apart(weekday(*d), input.long_day) != 1)
                .unwrap_or(0);
            easy_days.remove(drop);
        }
        let each = if easy_days.is_empty() {
            0.0
        } else {
            left / easy_days.len() as f64
        };
        for day in easy_days {
            let after_long = weekday(day) == (input.long_day + 1) % 7;
            let (kind, pace, km) = if after_long {
                (SessionKind::Recovery, recovery, each * 0.8)
            } else {
                (SessionKind::Easy, easy, each)
            };
            week_sessions.push(draft(day, kind, km.max(MIN_EASY_KM), pace, None));
        }
        if has_long
            && let Some(date) = dates
                .iter()
                .copied()
                .find(|d| weekday(*d) == input.long_day)
        {
            week_sessions.push(draft(date, SessionKind::Long, long_km, easy, None));
        }
        week_sessions.sort_by(|a, b| a.date.cmp(&b.date));
        sessions.extend(week_sessions);
    }

    let end = if input.target == TrainingTarget::Race {
        sessions
            .iter()
            .rev()
            .find(|s| s.kind == SessionKind::Race)
            .map(|s| s.date.clone())
    } else {
        None
    }
    .unwrap_or_else(|| format_date(start + Days::new(u64::from(input.weeks) * 7 - 1)));

    Ok(Preview {
        sessions,
        warnings,
        end,
        paces,
        race_pace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(weekly_km: f64, longest_km: f64, vdot: Option<f64>) -> Baseline {
        Baseline {
            weekly_km,
            weekly_km_long: weekly_km,
            runs_per_week: 3.0,
            longest_km,
            runs: 60,
            vdot,
            ..Baseline::default()
        }
    }

    fn race(km: f64, weeks: u32, days: &[u8]) -> PlanInput {
        PlanInput {
            name: "Plan".into(),
            target: TrainingTarget::Race,
            distance: Some(km),
            target_time: None,
            // A Monday.
            start: "2026-11-16".into(),
            weeks,
            days: days.to_vec(),
            long_day: 6,
        }
    }

    fn week_km(preview: &Preview, weeks: u32) -> Vec<f64> {
        (0..weeks)
            .map(|w| {
                preview
                    .sessions
                    .iter()
                    .filter(|s| s.week == w)
                    .map(|s| s.distance)
                    .sum()
            })
            .collect()
    }

    #[test]
    fn race_plans_go_from_base_to_taper() {
        let input = race(10.0, 10, &[1, 3, 4, 6]);
        let rules = rules(TrainingTarget::Race, 10.0);
        use Phase::*;
        assert_eq!(
            phases(&input, &rules),
            [
                Base, Base, Base, Build, Build, Build, Build, Peak, Peak, Taper
            ]
        );
        let half = race(21.1, 16, &[1, 3, 6]);
        let phases = phases(&half, &super::rules(TrainingTarget::Race, 21.1));
        assert_eq!(phases.iter().filter(|p| **p == Taper).count(), 2);
        assert_eq!(phases.last(), Some(&Taper));
    }

    #[test]
    fn volume_grows_by_at_most_ten_percent_with_easier_weeks() {
        let input = race(21.1, 16, &[1, 3, 4, 6]);
        let rules = rules(TrainingTarget::Race, 21.1);
        let phases = phases(&input, &rules);
        let volumes: Vec<f64> = volumes(&phases, TrainingTarget::Race, 25.0, 55.0, None)
            .into_iter()
            .map(Option::unwrap)
            .collect();
        assert_eq!(volumes[0], 25.0);
        let mut highest: f64 = 0.0;
        for (i, km) in volumes.iter().enumerate() {
            assert!(*km <= highest.max(25.0) * GROWTH + 1e-9, "week {i}: {km}");
            highest = highest.max(*km);
        }
        // Week 4 is easier than week 3, week 5 picks up where week 3 was.
        assert!(volumes[3] < volumes[2]);
        assert!(volumes[4] > volumes[2]);
        // The taper goes down to well below the peak.
        assert!(volumes[15] < volumes[14] && volumes[14] < highest);
    }

    #[test]
    fn plan_ends_with_the_race_on_the_long_run_day() {
        let input = race(10.0, 8, &[1, 3, 5, 6]);
        let preview = generate(&baseline(30.0, 14.0, Some(45.0)), &input, None).unwrap();
        let last = preview.sessions.last().unwrap();
        assert_eq!(last.kind, SessionKind::Race);
        assert_eq!(last.distance, 10.0);
        // Week 8 starts on Monday 4 January; its Sunday is the 10th.
        assert_eq!(last.date, "2027-01-10");
        assert_eq!(preview.end, "2027-01-10");
        assert!(last.pace.is_some());
        // No run on the day before the race.
        assert!(!preview.sessions.iter().any(|s| s.date == "2027-01-09"));
        assert!(preview.warnings.is_empty(), "{:?}", preview.warnings);
    }

    #[test]
    fn sessions_fall_on_the_chosen_days_and_hard_days_never_follow_each_other() {
        for days in [
            &[0, 1, 2, 3, 4, 5, 6][..],
            &[0, 1, 2, 6],
            &[1, 3, 6],
            &[5, 6],
            &[6],
        ] {
            let input = race(10.0, 10, days);
            let preview = generate(&baseline(25.0, 12.0, Some(45.0)), &input, None).unwrap();
            let mut hard: Vec<NaiveDate> = Vec::new();
            for session in &preview.sessions {
                let day = parse_date(&session.date).unwrap();
                assert!(days.contains(&weekday(day)), "{days:?}: {}", session.date);
                if matches!(
                    session.kind,
                    SessionKind::Long
                        | SessionKind::Tempo
                        | SessionKind::Intervals
                        | SessionKind::Race
                ) {
                    hard.push(day);
                }
            }
            for pair in hard.windows(2) {
                assert!((pair[1] - pair[0]).num_days() >= 2, "{days:?}: {pair:?}");
            }
        }
    }

    #[test]
    fn long_runs_grow_slowly_and_stay_under_the_cap() {
        let input = race(42.195, 18, &[1, 2, 4, 6]);
        let preview = generate(&baseline(50.0, 16.0, Some(48.0)), &input, None).unwrap();
        let longs: Vec<f64> = preview
            .sessions
            .iter()
            .filter(|s| s.kind == SessionKind::Long)
            .map(|s| s.distance)
            .collect();
        assert!(longs[0] <= 18.0);
        let mut longest: f64 = 16.0;
        for km in &longs {
            assert!(*km <= 32.0 && *km <= longest + LONG_STEP + 0.5, "{longs:?}");
            longest = longest.max(*km);
        }
        assert!(longest >= 28.0, "{longs:?}");
    }

    #[test]
    fn hard_sessions_fit_a_small_week() {
        // About 12 km a week on three days.
        let preview = generate(
            &baseline(12.0, 13.0, Some(38.0)),
            &race(10.0, 8, &[1, 3, 6]),
            None,
        )
        .unwrap();
        for week in 0..7 {
            let of_week: Vec<&SessionDraft> =
                preview.sessions.iter().filter(|s| s.week == week).collect();
            let long = of_week
                .iter()
                .find(|s| s.kind == SessionKind::Long)
                .unwrap();
            for session in &of_week {
                assert!(
                    session.distance <= long.distance,
                    "week {week}: {of_week:?}"
                );
            }
        }
    }

    #[test]
    fn fit_shortens_the_work() {
        let reps = Structure {
            reps: 6,
            metres: Some(1000),
            seconds: None,
            rest: 180,
        };
        assert_eq!(fit(reps, None, 3.5).reps, 3);
        assert_eq!(fit(reps, None, 10.0), reps);
        let timed = Structure {
            reps: 2,
            metres: None,
            seconds: Some(600),
            rest: 120,
        };
        // 2 × 10 min at 5:00/km is 4 km; 1.5 km leaves one rep of 7 min.
        assert_eq!(
            fit(timed, Some(300.0), 1.5),
            Structure {
                reps: 1,
                seconds: Some(420),
                ..timed
            }
        );
    }

    #[test]
    fn a_short_marathon_plan_warns() {
        let preview = generate(
            &baseline(8.0, 6.0, None),
            &race(42.195, 4, &[1, 3, 6]),
            None,
        )
        .unwrap();
        assert_eq!(
            preview.warnings,
            [
                PlanWarning::TooShort { weeks: 12 },
                PlanWarning::LowVolume { km: 30.0 },
                PlanWarning::NoPaces
            ]
        );
        assert!(preview.sessions.iter().all(|s| s.pace.is_none()));
    }

    #[test]
    fn a_target_time_sets_the_race_pace() {
        let mut input = race(10.0, 8, &[1, 3, 6]);
        input.target_time = Some(45 * 60);
        let preview = generate(&baseline(30.0, 12.0, Some(40.0)), &input, None).unwrap();
        assert_eq!(preview.race_pace, Some(270.0));
        // 45 min for 10 km is a VDOT of about 45, more than five above the current 40.
        assert!(matches!(
            preview.warnings[..],
            [PlanWarning::Ambitious { .. }]
        ));
        // Training paces come from the current VDOT.
        assert_eq!(preview.paces.unwrap().vdot, 40.0);
    }

    #[test]
    fn shape_plans_have_easier_weeks_and_one_tempo_run() {
        let input = PlanInput {
            target: TrainingTarget::Shape,
            distance: None,
            ..race(0.0, 8, &[1, 3, 6])
        };
        let preview = generate(&baseline(15.0, 8.0, Some(38.0)), &input, None).unwrap();
        let km = week_km(&preview, 8);
        assert!(km[3] < km[2] && km[7] < km[6], "{km:?}");
        for week in 0..8 {
            let tempo = preview
                .sessions
                .iter()
                .filter(|s| s.week == week && s.kind == SessionKind::Tempo)
                .count();
            assert_eq!(tempo, usize::from(week % 4 != 3), "week {week}");
        }
        assert_eq!(preview.end, "2027-01-10");
    }

    #[test]
    fn resume_starts_at_its_week_and_volume() {
        let input = race(10.0, 10, &[1, 3, 6]);
        let resume = Resume {
            week: 5,
            km: 20.0,
            longest_km: 10.0,
        };
        let preview = generate(&baseline(40.0, 15.0, Some(45.0)), &input, Some(resume)).unwrap();
        assert!(preview.sessions.iter().all(|s| s.week >= 5));
        let km = week_km(&preview, 10);
        assert!((km[5] - 20.0).abs() <= 3.0, "{km:?}");
    }

    #[test]
    fn checks_the_input() {
        let mut input = race(10.0, 8, &[1, 3, 6]);
        input.long_day = 2;
        assert!(validate(&input).is_err());
        input.long_day = 6;
        input.weeks = 0;
        assert!(validate(&input).is_err());
        input.weeks = 8;
        input.distance = None;
        assert!(validate(&input).is_err());
        input.distance = Some(10.0);
        assert!(validate(&input).is_ok());
    }
}
