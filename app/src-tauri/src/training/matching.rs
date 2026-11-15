//! Which run did which session, and whether it did what the session was for. Worked out on
//! every read like goal progress, so importing, editing or deleting a run leaves nothing stale;
//! only a link or unlink by hand is stored on the session.

use chrono::{DateTime, NaiveDate, TimeZone};
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use super::{
    generator::{SessionKind, Structure},
    paces::PaceRange,
};
use crate::{
    activity_type::ViewMode,
    analysis::{self, cardio},
    db::value::opt_f64,
    error::AppResult,
};

/// Below this share of the planned km a run only did part of the session.
const PARTIAL: f64 = 0.8;
/// Above this share an easy or long run went on too long.
const TOO_LONG: f64 = 1.3;
/// s/km faster than the easy range before an easy run counts as too hard.
const EASY_MARGIN: f64 = 10.0;
/// s/km slower than the target before the work of a tempo or interval run counts as too easy.
const WORK_MARGIN: f64 = 15.0;
/// A race is done from this share of its distance on (GPS measures a little short).
const RACE_DONE: f64 = 0.97;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Verdict {
    /// Still ahead, or today without a run yet.
    Planned,
    Done,
    /// Less than 80 % of the planned km.
    Partial,
    /// An easy, recovery or long run that was too fast, too long or in too high a zone.
    TooHard,
    /// The work of a tempo or interval run was slower than planned.
    TooEasy,
    Missed,
    Skipped,
}

/// What a session asks for, as far as matching goes.
#[derive(Debug, Clone)]
pub struct Planned {
    pub day: NaiveDate,
    pub kind: SessionKind,
    pub distance: f64,
    pub pace: Option<PaceRange>,
    pub structure: Option<Structure>,
    pub skipped: bool,
    /// Linked by hand.
    pub activity_id: Option<i64>,
    /// No automatic match.
    pub unlinked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MatchedRun {
    #[ts(type = "number")]
    pub id: i64,
    pub title: Option<String>,
    /// Unix timestamp.
    #[ts(type = "number")]
    pub date: i64,
    /// "YYYY-MM-DD", local time.
    pub day: String,
    /// km.
    pub distance: f64,
    /// s, without pauses.
    pub moving: f64,
    /// s/km.
    pub pace: Option<f64>,
    pub heart_rate: Option<f64>,
    #[serde(skip)]
    #[ts(skip)]
    pub local_day: NaiveDate,
}

/// How a run did against its session.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionCheck {
    pub verdict: Verdict,
    /// Done km over planned km.
    pub ratio: Option<f64>,
    /// s/km of the fastest kilometres, as many as the session's work.
    pub work_pace: Option<f64>,
    /// The average heart rate was above zone 2.
    pub above_zone_two: bool,
}

/// Runs between `from` and `to` (Unix timestamps), oldest first, with their local day in `tz`.
pub fn load_runs<Tz: TimeZone>(
    conn: &Connection,
    tz: &Tz,
    from: i64,
    to: i64,
) -> AppResult<Vec<MatchedRun>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.activity_type, a.date, a.distance, a.duration, a.title,
            r.avg_heart_rate, r.pause
         FROM activities a LEFT JOIN runs r ON r.activity_id = a.id
         WHERE a.date >= ?1 AND a.date < ?2 ORDER BY a.date, a.id",
    )?;
    let rows = stmt.query_map([from, to], |row| {
        let activity_type: Option<String> = row.get("activity_type")?;
        let date: i64 = row.get("date")?;
        let distance = opt_f64(row, "distance")?.unwrap_or(0.0);
        let duration = opt_f64(row, "duration")?.unwrap_or(0.0);
        let pause: Option<String> = row.get("pause")?;
        let moving = duration - cardio::pause_seconds(pause.as_deref().unwrap_or_default());
        let local_day = DateTime::from_timestamp(date, 0)
            .unwrap_or_default()
            .with_timezone(tz)
            .date_naive();
        let run = MatchedRun {
            id: row.get("id")?,
            title: row.get("title")?,
            date,
            day: local_day.format("%Y-%m-%d").to_string(),
            distance,
            moving,
            pace: (distance > 0.0 && moving > 0.0).then(|| moving / distance),
            heart_rate: opt_f64(row, "avg_heart_rate")?.filter(|hr| *hr > 0.0),
            local_day,
        };
        let running =
            ViewMode::from_activity_type(&activity_type.unwrap_or_default()) == ViewMode::Running;
        Ok(running.then_some(run))
    })?;
    Ok(rows
        .filter_map(Result::transpose)
        .collect::<Result<_, _>>()?)
}

fn closeness(session: &Planned, run: &MatchedRun) -> f64 {
    (run.distance - session.distance).abs()
}

/// The run (index into `runs`) each session took: its own link first, then a run on the same
/// day, then one a day before or after that no other session took.
pub fn assign(sessions: &[Planned], runs: &[MatchedRun]) -> Vec<Option<usize>> {
    let mut taken = vec![false; runs.len()];
    let mut matched: Vec<Option<usize>> = sessions
        .iter()
        .map(|s| {
            let index = s
                .activity_id
                .and_then(|id| runs.iter().position(|r| r.id == id))?;
            taken[index] = true;
            Some(index)
        })
        .collect();
    let mut order: Vec<usize> = (0..sessions.len()).collect();
    order.sort_by_key(|i| sessions[*i].day);
    for max_days in [0, 1] {
        for &i in &order {
            let session = &sessions[i];
            if matched[i].is_some()
                || session.skipped
                || session.unlinked
                || session.activity_id.is_some()
            {
                continue;
            }
            let best = runs
                .iter()
                .enumerate()
                .filter(|(r, run)| {
                    !taken[*r] && (run.local_day - session.day).num_days().abs() <= max_days
                })
                .min_by(|(_, a), (_, b)| {
                    let days = |run: &MatchedRun| (run.local_day - session.day).num_days().abs();
                    days(a)
                        .cmp(&days(b))
                        .then(closeness(session, a).total_cmp(&closeness(session, b)))
                });
            if let Some((r, _)) = best {
                taken[r] = true;
                matched[i] = Some(r);
            }
        }
    }
    matched
}

/// s/km of the fastest `count` per-km splits.
pub fn best_splits(conn: &Connection, activity_id: i64, count: usize) -> AppResult<Option<f64>> {
    let mut paces: Vec<f64> = analysis::segments(conn, activity_id)?
        .into_iter()
        // Only whole kilometres; the last, shorter split says little.
        .filter(|s| s.distance.is_none_or(|d| d >= 0.9))
        .filter_map(|s| s.pace)
        .filter(|p| *p > 0.0)
        .map(|p| p * 60.0)
        .collect();
    if paces.is_empty() {
        return Ok(None);
    }
    paces.sort_by(f64::total_cmp);
    let count = count.clamp(1, paces.len());
    Ok(Some(paces[..count].iter().sum::<f64>() / count as f64))
}

/// Number of kilometres the work of a session spans, for [`best_splits`].
pub fn work_splits(session: &Planned) -> usize {
    let pace = session.pace.map(|p| (p.fast + p.slow) / 2.0);
    session
        .structure
        .map_or(1.0, |s| s.work_km(pace))
        .floor()
        .max(1.0) as usize
}

/// How `run` did against `session`. `work_pace` is the pace of the fastest splits, for tempo
/// and interval runs; `zone_two` the upper bound of heart-rate zone 2.
pub fn check(
    session: &Planned,
    run: Option<&MatchedRun>,
    today: NaiveDate,
    work_pace: Option<f64>,
    zone_two: Option<i64>,
) -> SessionCheck {
    let Some(run) = run else {
        let verdict = if session.skipped {
            Verdict::Skipped
        } else if session.day < today {
            Verdict::Missed
        } else {
            Verdict::Planned
        };
        return SessionCheck {
            verdict,
            ratio: None,
            work_pace: None,
            above_zone_two: false,
        };
    };
    let ratio = (session.distance > 0.0).then(|| run.distance / session.distance);
    let above_zone_two = zone_two
        .zip(run.heart_rate)
        .is_some_and(|(bound, hr)| hr > bound as f64);
    let short = ratio.is_some_and(|r| r < PARTIAL);
    let verdict = match session.kind {
        SessionKind::Race => {
            if ratio.is_none_or(|r| r >= RACE_DONE) {
                Verdict::Done
            } else {
                Verdict::Partial
            }
        }
        _ if short => Verdict::Partial,
        SessionKind::Easy | SessionKind::Recovery | SessionKind::Long => {
            let too_fast = session
                .pace
                .zip(run.pace)
                .is_some_and(|(range, pace)| pace < range.fast - EASY_MARGIN);
            let too_long = session.kind != SessionKind::Long && ratio.is_some_and(|r| r > TOO_LONG);
            if too_fast || too_long || above_zone_two {
                Verdict::TooHard
            } else {
                Verdict::Done
            }
        }
        SessionKind::Tempo | SessionKind::Intervals => {
            let too_slow = session
                .pace
                .zip(work_pace)
                .is_some_and(|(range, pace)| pace > range.slow + WORK_MARGIN);
            if too_slow {
                Verdict::TooEasy
            } else {
                Verdict::Done
            }
        }
    };
    SessionCheck {
        verdict,
        ratio,
        work_pace,
        above_zone_two,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn session(date: &str, kind: SessionKind, distance: f64) -> Planned {
        Planned {
            day: day(date),
            kind,
            distance,
            pace: Some(PaceRange {
                fast: 300.0,
                slow: 330.0,
            }),
            structure: None,
            skipped: false,
            activity_id: None,
            unlinked: false,
        }
    }

    fn run(id: i64, date: &str, distance: f64, pace: f64) -> MatchedRun {
        MatchedRun {
            id,
            title: None,
            date: 0,
            day: date.into(),
            distance,
            moving: distance * pace,
            pace: Some(pace),
            heart_rate: Some(140.0),
            local_day: day(date),
        }
    }

    #[test]
    fn same_day_first_then_a_day_off() {
        let sessions = [
            session("2026-11-16", SessionKind::Easy, 8.0),
            session("2026-11-18", SessionKind::Easy, 8.0),
            session("2026-11-20", SessionKind::Long, 15.0),
        ];
        // Monday's run, two on Wednesday and Thursday's run that Friday's session takes.
        let runs = [
            run(1, "2026-11-16", 8.0, 320.0),
            run(2, "2026-11-18", 4.0, 320.0),
            run(3, "2026-11-18", 7.5, 320.0),
            run(4, "2026-11-19", 14.0, 320.0),
        ];
        assert_eq!(assign(&sessions, &runs), [Some(0), Some(2), Some(3)]);
    }

    #[test]
    fn a_run_is_never_taken_twice() {
        let sessions = [
            session("2026-11-16", SessionKind::Easy, 8.0),
            session("2026-11-17", SessionKind::Easy, 8.0),
        ];
        let runs = [run(1, "2026-11-17", 8.0, 320.0)];
        // Monday's session is earlier but Tuesday's has the run on its own day.
        assert_eq!(assign(&sessions, &runs), [None, Some(0)]);
    }

    #[test]
    fn links_by_hand_win() {
        let mut linked = session("2026-11-16", SessionKind::Easy, 8.0);
        linked.activity_id = Some(2);
        let mut unlinked = session("2026-11-17", SessionKind::Easy, 8.0);
        unlinked.unlinked = true;
        let mut skipped = session("2026-11-18", SessionKind::Easy, 8.0);
        skipped.skipped = true;
        let runs = [
            run(1, "2026-11-16", 8.0, 320.0),
            run(2, "2026-11-19", 8.0, 320.0),
            run(3, "2026-11-17", 8.0, 320.0),
            run(4, "2026-11-18", 8.0, 320.0),
        ];
        assert_eq!(
            assign(&[linked, unlinked, skipped], &runs),
            [Some(1), None, None]
        );
    }

    #[test]
    fn verdicts() {
        let today = day("2026-11-20");
        let easy = session("2026-11-16", SessionKind::Easy, 8.0);
        let verdict =
            |s: &Planned, r: Option<&MatchedRun>, work: Option<f64>, zone: Option<i64>| {
                check(s, r, today, work, zone).verdict
            };
        assert_eq!(
            verdict(&easy, Some(&run(1, "2026-11-16", 8.0, 315.0)), None, None),
            Verdict::Done
        );
        // Faster than 4:50/km on an easy day, too long, or above zone 2.
        assert_eq!(
            verdict(&easy, Some(&run(1, "2026-11-16", 8.0, 280.0)), None, None),
            Verdict::TooHard
        );
        assert_eq!(
            verdict(&easy, Some(&run(1, "2026-11-16", 12.0, 320.0)), None, None),
            Verdict::TooHard
        );
        assert_eq!(
            verdict(
                &easy,
                Some(&run(1, "2026-11-16", 8.0, 320.0)),
                None,
                Some(135)
            ),
            Verdict::TooHard
        );
        assert_eq!(
            verdict(&easy, Some(&run(1, "2026-11-16", 5.0, 320.0)), None, None),
            Verdict::Partial
        );
        assert_eq!(verdict(&easy, None, None, None), Verdict::Missed);
        let ahead = session("2026-11-20", SessionKind::Easy, 8.0);
        assert_eq!(verdict(&ahead, None, None, None), Verdict::Planned);

        let mut intervals = session("2026-11-17", SessionKind::Intervals, 10.0);
        intervals.pace = Some(PaceRange {
            fast: 230.0,
            slow: 240.0,
        });
        let done = run(2, "2026-11-17", 10.0, 290.0);
        assert_eq!(
            verdict(&intervals, Some(&done), Some(238.0), None),
            Verdict::Done
        );
        assert_eq!(
            verdict(&intervals, Some(&done), Some(262.0), None),
            Verdict::TooEasy
        );
        // Hard sessions go above zone 2 on purpose.
        assert_eq!(
            verdict(&intervals, Some(&done), Some(238.0), Some(135)),
            Verdict::Done
        );

        let race = session("2026-11-15", SessionKind::Race, 10.0);
        assert_eq!(
            verdict(&race, Some(&run(3, "2026-11-15", 9.8, 270.0)), None, None),
            Verdict::Done
        );
    }
}
