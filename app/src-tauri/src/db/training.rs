//! Training plans (`training_plans`) and their sessions (`training_sessions`). The plan and its
//! sessions are stored; which run did which session is worked out on read (`training::matching`).

use chrono::{Days, Local, NaiveDate, TimeZone};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    activity_type::ViewMode,
    analysis::comparison,
    db::{
        goals::{self, GoalInput, GoalMetric, GoalPeriod, GoalSpec, midnight, today_in},
        users,
    },
    error::{AppError, AppResult},
    training::{
        baseline::{self, Baseline},
        coach::{self, SessionCoachFacts, SessionFacts, WeekFacts},
        generator::{
            self, Phase, PlanInput, Preview, Resume, SessionDraft, SessionKind, Structure,
            TrainingTarget, format_date, parse_date,
        },
        matching::{self, MatchedRun, Planned, SessionCheck, Verdict},
        paces::{self, PaceRange, Paces},
    },
};

const WEEK: i64 = 7 * 86_400;
/// Weeks of runs the volume to go on from is taken from when adapting a plan.
const ADAPT_WEEKS: i64 = 3;
/// Days either side of a session runs are offered for linking.
const LINK_DAYS: u64 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrainingPlan {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub target: TrainingTarget,
    pub distance: Option<f64>,
    pub target_time: Option<u32>,
    /// "YYYY-MM-DD".
    pub start: String,
    /// "YYYY-MM-DD", the race day or the last day of the last week.
    pub end: String,
    pub weeks: u32,
    pub days: Vec<u8>,
    pub long_day: u8,
    /// Where the plan started from.
    pub baseline: Baseline,
    #[ts(type = "number | null")]
    pub goal_id: Option<i64>,
    pub archived: bool,
    /// Unix timestamp.
    #[ts(type = "number")]
    pub created: i64,
}

impl TrainingPlan {
    fn input(&self) -> PlanInput {
        PlanInput {
            name: self.name.clone(),
            target: self.target,
            distance: self.distance,
            target_time: self.target_time,
            start: self.start.clone(),
            weeks: self.weeks,
            days: self.days.clone(),
            long_day: self.long_day,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrainingSession {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub plan_id: i64,
    /// "YYYY-MM-DD".
    pub date: String,
    /// 0-based week of the plan.
    pub week: u32,
    pub phase: Phase,
    pub kind: SessionKind,
    /// km.
    pub distance: f64,
    pub pace: Option<PaceRange>,
    pub structure: Option<Structure>,
    pub note: String,
    /// Changed by hand; "Adapt plan" leaves it as it is.
    pub edited: bool,
    pub skipped: bool,
    /// Run linked by hand.
    #[ts(type = "number | null")]
    pub activity_id: Option<i64>,
    /// No run is matched automatically.
    pub unlinked: bool,
}

impl TrainingSession {
    fn planned(&self) -> AppResult<Planned> {
        Ok(Planned {
            day: parse_date(&self.date)?,
            kind: self.kind,
            distance: self.distance,
            pace: self.pace,
            structure: self.structure,
            skipped: self.skipped,
            activity_id: self.activity_id,
            unlinked: self.unlinked,
        })
    }
}

/// A session with the run that did it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionStatus {
    pub session: TrainingSession,
    pub run: Option<MatchedRun>,
    pub check: SessionCheck,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WeekSummary {
    pub week: u32,
    /// "YYYY-MM-DD".
    pub start: String,
    pub phase: Phase,
    pub planned_km: f64,
    pub done_km: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PlanStatus {
    Upcoming,
    Active,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanSummary {
    pub plan: TrainingPlan,
    pub status: PlanStatus,
    pub sessions: u32,
    /// Sessions with a matched run.
    pub trained: u32,
    /// Sessions done as planned.
    pub done: u32,
    pub missed: u32,
    pub planned_km: f64,
    pub done_km: f64,
    /// The first session from today on.
    pub next: Option<TrainingSession>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanGoal {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanDetail {
    pub summary: PlanSummary,
    pub sessions: Vec<SessionStatus>,
    pub weeks: Vec<WeekSummary>,
    /// Paces from the plan's baseline, or from the target time without one.
    pub paces: Option<Paces>,
    /// The matching goal, while it exists.
    pub goal: Option<PlanGoal>,
}

/// A session of a plan, for the dashboard and the activity panel.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlanSession {
    #[ts(type = "number")]
    pub plan_id: i64,
    pub plan_name: String,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionInput {
    pub date: String,
    pub kind: SessionKind,
    pub distance: f64,
    pub pace: Option<PaceRange>,
    pub structure: Option<Structure>,
    pub note: String,
}

fn enum_name<T: Serialize>(value: &T) -> AppResult<String> {
    Ok(serde_json::to_value(value)?
        .as_str()
        .unwrap_or_default()
        .to_string())
}

fn parse_enum<T: for<'de> Deserialize<'de>>(name: &str) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(name.into())).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn parse_json<T: for<'de> Deserialize<'de>>(text: &str) -> rusqlite::Result<T> {
    serde_json::from_str(text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

const PLAN_COLUMNS: &str = "id, name, target, distance, target_time, start_date, weeks, days, \
    long_day, baseline, goal_id, archived, created, \
    (SELECT MAX(date) FROM training_sessions s \
     WHERE s.plan_id = training_plans.id AND s.kind = 'race') AS race_date";

fn plan_from_row(row: &Row) -> rusqlite::Result<TrainingPlan> {
    let start: String = row.get("start_date")?;
    let weeks: u32 = row.get("weeks")?;
    let race_date: Option<String> = row.get("race_date")?;
    let end = race_date.unwrap_or_else(|| {
        parse_date(&start).map_or_else(
            |_| start.clone(),
            |day| format_date(day + Days::new(u64::from(weeks) * 7 - 1)),
        )
    });
    Ok(TrainingPlan {
        id: row.get("id")?,
        name: row.get("name")?,
        target: parse_enum(&row.get::<_, String>("target")?)?,
        distance: row.get("distance")?,
        target_time: row.get("target_time")?,
        start,
        end,
        weeks,
        days: parse_json(&row.get::<_, String>("days")?)?,
        long_day: row.get("long_day")?,
        baseline: parse_json(&row.get::<_, String>("baseline")?).unwrap_or_default(),
        goal_id: row.get("goal_id")?,
        archived: row.get("archived")?,
        created: row.get("created")?,
    })
}

fn plans(conn: &Connection) -> AppResult<Vec<TrainingPlan>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PLAN_COLUMNS} FROM training_plans ORDER BY start_date DESC, id DESC"
    ))?;
    Ok(stmt
        .query_map([], plan_from_row)?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<TrainingPlan> {
    conn.query_row(
        &format!("SELECT {PLAN_COLUMNS} FROM training_plans WHERE id = ?1"),
        [id],
        plan_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("training plan {id}")))
}

const SESSION_COLUMNS: &str = "id, plan_id, date, week, phase, kind, distance, pace_fast, \
    pace_slow, structure, note, edited, skipped, activity_id, unlinked";

fn session_from_row(row: &Row) -> rusqlite::Result<TrainingSession> {
    let fast: Option<f64> = row.get("pace_fast")?;
    let slow: Option<f64> = row.get("pace_slow")?;
    let structure: Option<String> = row.get("structure")?;
    Ok(TrainingSession {
        id: row.get("id")?,
        plan_id: row.get("plan_id")?,
        date: row.get("date")?,
        week: row.get("week")?,
        phase: parse_enum(&row.get::<_, String>("phase")?)?,
        kind: parse_enum(&row.get::<_, String>("kind")?)?,
        distance: row.get("distance")?,
        pace: fast.zip(slow).map(|(fast, slow)| PaceRange { fast, slow }),
        structure: structure.as_deref().map(parse_json).transpose()?,
        note: row.get("note")?,
        edited: row.get("edited")?,
        skipped: row.get("skipped")?,
        activity_id: row.get("activity_id")?,
        unlinked: row.get("unlinked")?,
    })
}

fn sessions(conn: &Connection, plan_id: i64) -> AppResult<Vec<TrainingSession>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SESSION_COLUMNS} FROM training_sessions WHERE plan_id = ?1 ORDER BY date, id"
    ))?;
    Ok(stmt
        .query_map([plan_id], session_from_row)?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn get_session(conn: &Connection, id: i64) -> AppResult<TrainingSession> {
    conn.query_row(
        &format!("SELECT {SESSION_COLUMNS} FROM training_sessions WHERE id = ?1"),
        [id],
        session_from_row,
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("training session {id}")))
}

fn insert_session(conn: &Connection, plan_id: i64, draft: &SessionDraft) -> AppResult<()> {
    conn.execute(
        "INSERT INTO training_sessions (plan_id, date, week, phase, kind, distance, pace_fast,
            pace_slow, structure)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            plan_id,
            draft.date,
            draft.week,
            enum_name(&draft.phase)?,
            enum_name(&draft.kind)?,
            draft.distance,
            draft.pace.map(|p| p.fast),
            draft.pace.map(|p| p.slow),
            draft
                .structure
                .map(|s| serde_json::to_string(&s))
                .transpose()?,
        ],
    )?;
    Ok(())
}

/// Where a plan would start from now.
pub fn current_baseline(conn: &Connection, now: i64) -> AppResult<Baseline> {
    baseline::baseline(conn, now)
}

/// The plan `input` would get, without saving it.
pub fn preview(conn: &Connection, input: &PlanInput, now: i64) -> AppResult<Preview> {
    generator::generate(&current_baseline(conn, now)?, input, None)
}

/// The goal made along with a plan: the race distance in one run by the race day, or the
/// plan's average weekly km every week.
fn matching_goal(plan: &TrainingPlan, preview: &Preview) -> GoalInput {
    let spec = match plan.target {
        TrainingTarget::Race => GoalSpec {
            metric: GoalMetric::Distance,
            target: plan.distance.unwrap_or(5.0),
            per_activity: true,
            max_duration: plan.target_time,
            period: GoalPeriod::Open,
            days: None,
            repeat: false,
        },
        TrainingTarget::Shape | TrainingTarget::Fitness => {
            let km: f64 = preview.sessions.iter().map(|s| s.distance).sum();
            GoalSpec {
                metric: GoalMetric::Distance,
                target: (km / f64::from(plan.weeks.max(1))).round().max(1.0),
                per_activity: false,
                max_duration: None,
                period: GoalPeriod::Week,
                days: None,
                repeat: true,
            }
        }
    };
    GoalInput {
        name: plan.name.clone(),
        view: ViewMode::Running,
        spec,
        start: plan.start.clone(),
        end: Some(plan.end.clone()),
    }
}

pub fn create(
    conn: &Connection,
    input: &PlanInput,
    create_goal: bool,
    now: i64,
) -> AppResult<TrainingPlan> {
    let baseline = current_baseline(conn, now)?;
    let preview = generator::generate(&baseline, input, None)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO training_plans (name, target, distance, target_time, start_date, weeks, days,
            long_day, baseline, archived, created)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10)",
        params![
            input.name.trim(),
            enum_name(&input.target)?,
            input.distance,
            input.target_time,
            input.start,
            input.weeks,
            serde_json::to_string(&input.days)?,
            input.long_day,
            serde_json::to_string(&baseline)?,
            now,
        ],
    )?;
    let id = tx.last_insert_rowid();
    for draft in &preview.sessions {
        insert_session(&tx, id, draft)?;
    }
    if create_goal {
        let plan = get(&tx, id)?;
        let goal = goals::create(&tx, &matching_goal(&plan, &preview), now)?;
        tx.execute(
            "UPDATE training_plans SET goal_id = ?2 WHERE id = ?1",
            params![id, goal.id],
        )?;
    }
    tx.commit()?;
    get(conn, id)
}

pub fn rename(conn: &Connection, id: i64, name: &str) -> AppResult<TrainingPlan> {
    if name.trim().is_empty() {
        return Err(AppError::Invalid("a plan needs a name".into()));
    }
    conn.execute(
        "UPDATE training_plans SET name = ?2 WHERE id = ?1",
        params![id, name.trim()],
    )?;
    get(conn, id)
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> AppResult<TrainingPlan> {
    conn.execute(
        "UPDATE training_plans SET archived = ?2 WHERE id = ?1",
        params![id, archived],
    )?;
    get(conn, id)
}

/// Deletes the plan with its sessions and notes; a matching goal stays.
pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM training_sessions WHERE plan_id = ?1", [id])?;
    tx.execute("DELETE FROM training_notes WHERE plan_id = ?1", [id])?;
    tx.execute("DELETE FROM training_plans WHERE id = ?1", [id])?;
    tx.commit()?;
    Ok(())
}

/// Upper bound of heart-rate zone 2, once the user's zone method has zones.
fn zone_two(conn: &Connection) -> AppResult<Option<i64>> {
    Ok(users::zones(conn)?.map(|z| z.upper[1]))
}

/// Sessions with the runs that did them.
fn statuses<Tz: TimeZone>(
    conn: &Connection,
    tz: &Tz,
    sessions: Vec<TrainingSession>,
    today: NaiveDate,
    zone_two: Option<i64>,
) -> AppResult<Vec<SessionStatus>> {
    let planned: Vec<Planned> = sessions
        .iter()
        .map(TrainingSession::planned)
        .collect::<AppResult<_>>()?;
    let (Some(first), Some(last)) = (
        planned.iter().map(|p| p.day).min(),
        planned.iter().map(|p| p.day).max(),
    ) else {
        return Ok(Vec::new());
    };
    let from = midnight(tz, first - Days::new(1));
    let to = midnight(tz, last + Days::new(2));
    let mut runs = matching::load_runs(conn, tz, from, to)?;
    // A run linked by hand may lie outside the window.
    for id in sessions.iter().filter_map(|s| s.activity_id) {
        if !runs.iter().any(|r| r.id == id) {
            runs.extend(linked_run(conn, tz, id)?);
        }
    }
    let assigned = matching::assign(&planned, &runs);
    sessions
        .into_iter()
        .zip(planned)
        .zip(assigned)
        .map(|((session, planned), index)| {
            let run = index.map(|i| runs[i].clone());
            let work_pace = match (&run, session.kind) {
                (Some(run), SessionKind::Tempo | SessionKind::Intervals) => {
                    matching::best_splits(conn, run.id, matching::work_splits(&planned))?
                }
                _ => None,
            };
            let check = matching::check(&planned, run.as_ref(), today, work_pace, zone_two);
            Ok(SessionStatus {
                session,
                run,
                check,
            })
        })
        .collect()
}

fn linked_run<Tz: TimeZone>(conn: &Connection, tz: &Tz, id: i64) -> AppResult<Option<MatchedRun>> {
    let date: Option<i64> = conn
        .query_row("SELECT date FROM activities WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .optional()?
        .flatten();
    let Some(date) = date else { return Ok(None) };
    Ok(matching::load_runs(conn, tz, date, date + 1)?
        .into_iter()
        .find(|r| r.id == id))
}

fn status_of(plan: &TrainingPlan, today: NaiveDate) -> PlanStatus {
    let day = format_date(today);
    if day < plan.start {
        PlanStatus::Upcoming
    } else if day > plan.end {
        PlanStatus::Finished
    } else {
        PlanStatus::Active
    }
}

fn summary(plan: TrainingPlan, statuses: &[SessionStatus], today: NaiveDate) -> PlanSummary {
    let count = |verdicts: &[Verdict]| {
        statuses
            .iter()
            .filter(|s| verdicts.contains(&s.check.verdict))
            .count() as u32
    };
    let day = format_date(today);
    PlanSummary {
        status: status_of(&plan, today),
        sessions: statuses.len() as u32,
        trained: statuses.iter().filter(|s| s.run.is_some()).count() as u32,
        done: count(&[Verdict::Done]),
        missed: count(&[Verdict::Missed]),
        planned_km: statuses.iter().map(|s| s.session.distance).sum(),
        done_km: statuses
            .iter()
            .filter_map(|s| s.run.as_ref())
            .map(|r| r.distance)
            .sum(),
        next: statuses
            .iter()
            .find(|s| s.session.date >= day && s.run.is_none() && !s.session.skipped)
            .map(|s| s.session.clone()),
        plan,
    }
}

fn list_in<Tz: TimeZone>(
    conn: &Connection,
    tz: &Tz,
    now: i64,
    include_archived: bool,
) -> AppResult<Vec<PlanSummary>> {
    let today = today_in(tz, now);
    let zone = zone_two(conn)?;
    plans(conn)?
        .into_iter()
        .filter(|plan| include_archived || !plan.archived)
        .map(|plan| {
            let statuses = statuses(conn, tz, sessions(conn, plan.id)?, today, zone)?;
            Ok(summary(plan, &statuses, today))
        })
        .collect()
}

pub fn list(conn: &Connection, now: i64, include_archived: bool) -> AppResult<Vec<PlanSummary>> {
    list_in(conn, &Local, now, include_archived)
}

fn detail_in<Tz: TimeZone>(conn: &Connection, tz: &Tz, id: i64, now: i64) -> AppResult<PlanDetail> {
    let plan = get(conn, id)?;
    let today = today_in(tz, now);
    let statuses = statuses(conn, tz, sessions(conn, id)?, today, zone_two(conn)?)?;
    let start = parse_date(&plan.start)?;
    let weeks = (0..plan.weeks)
        .map(|week| {
            let of_week: Vec<&SessionStatus> =
                statuses.iter().filter(|s| s.session.week == week).collect();
            WeekSummary {
                week,
                start: format_date(start + Days::new(u64::from(week) * 7)),
                phase: of_week.first().map_or(Phase::Base, |s| s.session.phase),
                planned_km: of_week.iter().map(|s| s.session.distance).sum(),
                done_km: of_week
                    .iter()
                    .filter_map(|s| s.run.as_ref())
                    .map(|r| r.distance)
                    .sum(),
            }
        })
        .collect();
    let paces = generator::generate(&plan.baseline, &plan.input(), None)
        .ok()
        .and_then(|p| p.paces)
        .or_else(|| plan.baseline.vdot.map(paces::paces));
    let goal = plan
        .goal_id
        .and_then(|id| goals::get(conn, id).ok())
        .map(|g| PlanGoal {
            id: g.id,
            name: g.name,
        });
    Ok(PlanDetail {
        summary: summary(plan, &statuses, today),
        sessions: statuses,
        weeks,
        paces,
        goal,
    })
}

pub fn detail(conn: &Connection, id: i64, now: i64) -> AppResult<PlanDetail> {
    detail_in(conn, &Local, id, now)
}

/// Rebuilds the sessions from today on that weren't changed, linked or skipped by hand, from
/// the runs of the last weeks and the plan's phases. Sessions before today stay.
fn adapt_in<Tz: TimeZone>(conn: &Connection, tz: &Tz, id: i64, now: i64) -> AppResult<PlanDetail> {
    let plan = get(conn, id)?;
    let today = today_in(tz, now);
    let start = parse_date(&plan.start)?;
    let day = format_date(today);
    if day > plan.end {
        return Err(AppError::Invalid("the plan is over".into()));
    }
    let week = (today.max(start) - start).num_days() as u32 / 7;
    let input = plan.input();

    // Go on with what was run lately, but not with more than the plan had for this week.
    let original = generator::generate(&plan.baseline, &input, None)?;
    let planned_km: f64 = original
        .sessions
        .iter()
        .filter(|s| s.week == week)
        .map(|s| s.distance)
        .sum();
    let current = current_baseline(conn, now)?;
    let recent: f64 = matching::load_runs(conn, tz, now - ADAPT_WEEKS * WEEK, now)?
        .iter()
        .map(|r| r.distance)
        .sum::<f64>()
        / ADAPT_WEEKS as f64;
    let resume = Resume {
        week,
        km: planned_km.min((recent * 1.1).max(10.0)),
        longest_km: current.longest_km,
    };
    let preview = generator::generate(&current, &input, Some(resume))?;

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM training_sessions
         WHERE plan_id = ?1 AND date >= ?2 AND edited = 0 AND skipped = 0
            AND activity_id IS NULL AND unlinked = 0",
        params![id, day],
    )?;
    let kept: Vec<String> = sessions(&tx, id)?
        .into_iter()
        .filter(|s| s.date >= day)
        .map(|s| s.date)
        .collect();
    for draft in preview
        .sessions
        .iter()
        .filter(|d| d.date >= day && !kept.contains(&d.date))
    {
        insert_session(&tx, id, draft)?;
    }
    tx.commit()?;
    detail_in(conn, tz, id, now)
}

pub fn adapt(conn: &Connection, id: i64, now: i64) -> AppResult<PlanDetail> {
    adapt_in(conn, &Local, id, now)
}

/// Checks a session input; returns its week in `plan`.
fn session_week(plan: &TrainingPlan, input: &SessionInput) -> AppResult<u32> {
    let day = parse_date(&input.date)?;
    let start = parse_date(&plan.start)?;
    if !(input.distance.is_finite() && input.distance > 0.0 && input.distance <= 200.0) {
        return Err(AppError::Invalid("the distance must be above zero".into()));
    }
    if input
        .pace
        .is_some_and(|p| !(p.fast > 0.0 && p.slow >= p.fast))
    {
        return Err(AppError::Invalid("the pace range is not valid".into()));
    }
    if day < start || format_date(day) > plan.end {
        return Err(AppError::Invalid(
            "the date must lie within the plan".into(),
        ));
    }
    Ok(((day - start).num_days() as u32 / 7).min(plan.weeks.saturating_sub(1)))
}

fn phase_of_week(conn: &Connection, plan_id: i64, week: u32) -> AppResult<Phase> {
    let phase: Option<String> = conn
        .query_row(
            "SELECT phase FROM training_sessions WHERE plan_id = ?1 AND week = ?2 LIMIT 1",
            params![plan_id, week],
            |row| row.get(0),
        )
        .optional()?;
    Ok(phase
        .and_then(|p| parse_enum(&p).ok())
        .unwrap_or(Phase::Build))
}

pub fn update_session(
    conn: &Connection,
    id: i64,
    input: &SessionInput,
) -> AppResult<TrainingSession> {
    let session = get_session(conn, id)?;
    let plan = get(conn, session.plan_id)?;
    let week = session_week(&plan, input)?;
    conn.execute(
        "UPDATE training_sessions SET date = ?2, week = ?3, kind = ?4, distance = ?5,
            pace_fast = ?6, pace_slow = ?7, structure = ?8, note = ?9, edited = 1
         WHERE id = ?1",
        params![
            id,
            input.date,
            week,
            enum_name(&input.kind)?,
            input.distance,
            input.pace.map(|p| p.fast),
            input.pace.map(|p| p.slow),
            input
                .structure
                .map(|s| serde_json::to_string(&s))
                .transpose()?,
            input.note.trim(),
        ],
    )?;
    get_session(conn, id)
}

pub fn add_session(
    conn: &Connection,
    plan_id: i64,
    input: &SessionInput,
) -> AppResult<TrainingSession> {
    let plan = get(conn, plan_id)?;
    let week = session_week(&plan, input)?;
    conn.execute(
        "INSERT INTO training_sessions (plan_id, date, week, phase, kind, distance, pace_fast,
            pace_slow, structure, note, edited)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1)",
        params![
            plan_id,
            input.date,
            week,
            enum_name(&phase_of_week(conn, plan_id, week)?)?,
            enum_name(&input.kind)?,
            input.distance,
            input.pace.map(|p| p.fast),
            input.pace.map(|p| p.slow),
            input
                .structure
                .map(|s| serde_json::to_string(&s))
                .transpose()?,
            input.note.trim(),
        ],
    )?;
    get_session(conn, conn.last_insert_rowid())
}

pub fn delete_session(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM training_sessions WHERE id = ?1", [id])?;
    conn.execute("DELETE FROM training_notes WHERE session_id = ?1", [id])?;
    Ok(())
}

pub fn skip_session(conn: &Connection, id: i64, skipped: bool) -> AppResult<TrainingSession> {
    conn.execute(
        "UPDATE training_sessions SET skipped = ?2 WHERE id = ?1",
        params![id, skipped],
    )?;
    get_session(conn, id)
}

/// Links a run to the session by hand, or with `None` goes back to automatic matching;
/// `unlinked` stops automatic matching for the session.
pub fn link_session(
    conn: &Connection,
    id: i64,
    activity_id: Option<i64>,
    unlinked: bool,
) -> AppResult<TrainingSession> {
    conn.execute(
        "UPDATE training_sessions SET activity_id = ?2, unlinked = ?3 WHERE id = ?1",
        params![id, activity_id, unlinked && activity_id.is_none()],
    )?;
    get_session(conn, id)
}

fn link_candidates_in<Tz: TimeZone>(
    conn: &Connection,
    tz: &Tz,
    id: i64,
) -> AppResult<Vec<MatchedRun>> {
    let day = parse_date(&get_session(conn, id)?.date)?;
    matching::load_runs(
        conn,
        tz,
        midnight(tz, day - Days::new(LINK_DAYS)),
        midnight(tz, day + Days::new(LINK_DAYS + 1)),
    )
}

/// Runs a few days around the session, to link one by hand.
pub fn link_candidates(conn: &Connection, id: i64) -> AppResult<Vec<MatchedRun>> {
    link_candidates_in(conn, &Local, id)
}

/// Statuses of the sessions of every plan that isn't archived, with `pick` choosing which.
fn plan_sessions_in<Tz: TimeZone>(
    conn: &Connection,
    tz: &Tz,
    now: i64,
    pick: impl Fn(&TrainingPlan, &SessionStatus) -> bool,
) -> AppResult<Vec<PlanSession>> {
    let today = today_in(tz, now);
    let zone = zone_two(conn)?;
    let mut found = Vec::new();
    for plan in plans(conn)?.into_iter().filter(|p| !p.archived) {
        for status in statuses(conn, tz, sessions(conn, plan.id)?, today, zone)? {
            if pick(&plan, &status) {
                found.push(PlanSession {
                    plan_id: plan.id,
                    plan_name: plan.name.clone(),
                    status,
                });
            }
        }
    }
    found.sort_by(|a, b| a.status.session.date.cmp(&b.status.session.date));
    Ok(found)
}

/// Sessions from `from` to `to` ("YYYY-MM-DD", both included) of the plans in use.
pub fn sessions_between(
    conn: &Connection,
    from: &str,
    to: &str,
    now: i64,
) -> AppResult<Vec<PlanSession>> {
    parse_date(from)?;
    parse_date(to)?;
    plan_sessions_in(conn, &Local, now, |plan, status| {
        plan.end.as_str() >= from
            && plan.start.as_str() <= to
            && (from..=to).contains(&status.session.date.as_str())
    })
}

/// The session the activity did, if any.
pub fn activity_session(
    conn: &Connection,
    activity_id: i64,
    now: i64,
) -> AppResult<Option<PlanSession>> {
    Ok(plan_sessions_in(conn, &Local, now, |_, status| {
        status.run.as_ref().is_some_and(|r| r.id == activity_id)
    })?
    .into_iter()
    .next())
}

/// Text the local AI model wrote about a plan week or a session.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TrainingNote {
    pub model: String,
    pub text: String,
    /// Unix timestamp.
    #[ts(type = "number")]
    pub created: i64,
}

/// The note on `week` of the plan, or on `session_id`.
pub fn get_note(
    conn: &Connection,
    plan_id: i64,
    week: Option<u32>,
    session_id: Option<i64>,
) -> AppResult<Option<TrainingNote>> {
    Ok(conn
        .query_row(
            "SELECT model, text, created FROM training_notes
             WHERE plan_id = ?1 AND week IS ?2 AND session_id IS ?3
             ORDER BY id DESC LIMIT 1",
            params![plan_id, week, session_id],
            |row| {
                Ok(TrainingNote {
                    model: row.get("model")?,
                    text: row.get("text")?,
                    created: row.get("created")?,
                })
            },
        )
        .optional()?)
}

pub fn save_note(
    conn: &Connection,
    plan_id: i64,
    week: Option<u32>,
    session_id: Option<i64>,
    note: &TrainingNote,
) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM training_notes WHERE plan_id = ?1 AND week IS ?2 AND session_id IS ?3",
        params![plan_id, week, session_id],
    )?;
    tx.execute(
        "INSERT INTO training_notes (plan_id, week, session_id, model, text, created)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            plan_id,
            week,
            session_id,
            note.model,
            note.text,
            note.created
        ],
    )?;
    tx.commit()?;
    Ok(())
}

/// What the AI model gets to explain `week` (0-based) of a plan.
pub fn week_facts(conn: &Connection, plan_id: i64, week: u32, now: i64) -> AppResult<WeekFacts> {
    let detail = detail(conn, plan_id, now)?;
    let of_week = |w: u32| -> Vec<SessionFacts> {
        detail
            .sessions
            .iter()
            .filter(|s| s.session.week == w)
            .map(coach::session_facts)
            .collect()
    };
    let phase = detail
        .weeks
        .iter()
        .find(|w| w.week == week)
        .map(|w| w.phase)
        .ok_or_else(|| AppError::NotFound(format!("week {week} of plan {plan_id}")))?;
    Ok(WeekFacts {
        plan: coach::plan_facts(&detail.summary.plan),
        week: week + 1,
        phase: enum_name(&phase)?,
        sessions: of_week(week),
        previous_week: week.checked_sub(1).map(of_week).unwrap_or_default(),
    })
}

/// What the AI model gets to comment on a session and its run, with the plan's id.
pub fn session_coach_facts(
    conn: &Connection,
    session_id: i64,
    now: i64,
    imports_dir: &std::path::Path,
    period_weeks: u8,
) -> AppResult<(i64, SessionCoachFacts)> {
    let session = get_session(conn, session_id)?;
    let detail = detail(conn, session.plan_id, now)?;
    let status = detail
        .sessions
        .iter()
        .find(|s| s.session.id == session_id)
        .ok_or_else(|| AppError::NotFound(format!("training session {session_id}")))?;
    let run = status
        .run
        .as_ref()
        .ok_or_else(|| AppError::Invalid("the session has no run yet".into()))?;
    Ok((
        session.plan_id,
        SessionCoachFacts {
            plan: coach::plan_facts(&detail.summary.plan),
            session: coach::session_facts(status),
            run: comparison::comparison(conn, run.id, imports_dir, period_weeks)?,
        },
    ))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    fn at(day: &str, hour: u32) -> i64 {
        parse_date(day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    fn input() -> PlanInput {
        PlanInput {
            name: "10k".into(),
            target: TrainingTarget::Race,
            distance: Some(10.0),
            target_time: None,
            start: "2026-11-02".into(),
            weeks: 8,
            days: vec![1, 3, 6],
            long_day: 6,
        }
    }

    /// A plan from Monday 2 November 2026 on.
    fn seeded() -> (Db, i64) {
        let db = Db::open_in_memory().unwrap();
        let id = {
            let conn = db.conn().unwrap();
            // Three runs of 8 km a week before the plan.
            for i in 1..=24 {
                insert_activity(
                    &conn,
                    i,
                    "Running",
                    at("2026-11-01", 8) - i * 2 * 86_400,
                    8.0,
                    "Run",
                );
            }
            create(&conn, &input(), true, at("2026-11-01", 12))
                .unwrap()
                .id
        };
        (db, id)
    }

    fn detail_at(conn: &Connection, id: i64, now: i64) -> PlanDetail {
        detail_in(conn, &Utc, id, now).unwrap()
    }

    #[test]
    fn creates_the_plan_with_its_sessions_and_goal() {
        let (db, id) = seeded();
        let conn = db.conn().unwrap();
        let plan = get(&conn, id).unwrap();
        assert_eq!(plan.end, "2026-12-27");
        assert_eq!(plan.baseline.runs, 24);
        let goal = goals::get(&conn, plan.goal_id.unwrap()).unwrap();
        assert_eq!(goal.spec.target, 10.0);
        assert_eq!(goal.end.as_deref(), Some("2026-12-27"));
        let detail = detail_at(&conn, id, at("2026-11-01", 12));
        assert_eq!(detail.weeks.len(), 8);
        assert_eq!(detail.summary.status, PlanStatus::Upcoming);
        assert!(
            detail
                .sessions
                .iter()
                .all(|s| s.check.verdict == Verdict::Planned)
        );
        assert_eq!(detail.goal.unwrap().name, "10k");
    }

    #[test]
    fn matches_runs_and_counts_missed_sessions() {
        let (db, id) = seeded();
        let conn = db.conn().unwrap();
        // Tuesday 3 November: an easy run on the right day. Thursday's session is missed.
        insert_activity(&conn, 100, "Running", at("2026-11-03", 7), 6.0, "Tuesday");
        let detail = detail_at(&conn, id, at("2026-11-07", 12));
        let first = &detail.sessions[0];
        assert_eq!(first.session.date, "2026-11-03");
        assert_eq!(first.run.as_ref().unwrap().id, 100);
        assert_eq!(detail.sessions[1].check.verdict, Verdict::Missed);
        assert_eq!(detail.summary.status, PlanStatus::Active);
        assert_eq!(detail.summary.missed, 1);
        assert_eq!(detail.weeks[0].done_km, 6.0);

        let found = activity_session(&conn, 100, at("2026-11-07", 12))
            .unwrap()
            .unwrap();
        assert_eq!(found.status.session.id, first.session.id);
    }

    #[test]
    fn adapt_keeps_past_and_edited_sessions() {
        let (db, id) = seeded();
        let conn = db.conn().unwrap();
        let before = sessions(&conn, id).unwrap();
        let past = before[0].clone();
        // An edited session in week 5.
        let edited = before.iter().find(|s| s.week == 5).unwrap().clone();
        update_session(
            &conn,
            edited.id,
            &SessionInput {
                date: edited.date.clone(),
                kind: SessionKind::Easy,
                distance: 4.0,
                pace: None,
                structure: None,
                note: "Knee".into(),
            },
        )
        .unwrap();
        adapt_in(&conn, &Utc, id, at("2026-11-20", 12)).unwrap();
        let after = sessions(&conn, id).unwrap();
        assert!(after.contains(&past));
        let kept = after.iter().find(|s| s.id == edited.id).unwrap();
        assert_eq!((kept.distance, kept.note.as_str()), (4.0, "Knee"));
        assert_eq!(after.iter().filter(|s| s.date == edited.date).count(), 1);
        // The race is still there.
        assert_eq!(after.last().unwrap().kind, SessionKind::Race);
    }

    #[test]
    fn link_unlink_and_skip() {
        let (db, id) = seeded();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 100, "Running", at("2026-11-03", 7), 6.0, "Tuesday");
        let first = sessions(&conn, id).unwrap()[0].clone();
        link_session(&conn, first.id, None, true).unwrap();
        let now = at("2026-11-04", 12);
        assert!(detail_at(&conn, id, now).sessions[0].run.is_none());
        let candidates = link_candidates_in(&conn, &Utc, first.id).unwrap();
        assert_eq!(candidates.len(), 1);
        link_session(&conn, first.id, Some(100), false).unwrap();
        assert_eq!(
            detail_at(&conn, id, now).sessions[0]
                .run
                .as_ref()
                .unwrap()
                .id,
            100
        );
        skip_session(&conn, sessions(&conn, id).unwrap()[1].id, true).unwrap();
        delete(&conn, id).unwrap();
        assert!(sessions(&conn, id).unwrap().is_empty());
        // The goal stays.
        assert_eq!(goals::list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn keeps_one_note_per_week_or_session() {
        let (db, id) = seeded();
        let conn = db.conn().unwrap();
        let note = |text: &str| TrainingNote {
            model: "m".into(),
            text: text.into(),
            created: 1,
        };
        save_note(&conn, id, Some(0), None, &note("first")).unwrap();
        save_note(&conn, id, Some(0), None, &note("again")).unwrap();
        save_note(&conn, id, None, Some(7), &note("session")).unwrap();
        assert_eq!(
            get_note(&conn, id, Some(0), None).unwrap().unwrap().text,
            "again"
        );
        assert_eq!(
            get_note(&conn, id, None, Some(7)).unwrap().unwrap().text,
            "session"
        );
        assert!(get_note(&conn, id, Some(1), None).unwrap().is_none());

        let facts = week_facts(&conn, id, 1, at("2026-11-12", 12)).unwrap();
        assert_eq!(facts.week, 2);
        assert_eq!(facts.phase, "base");
        assert!(!facts.sessions.is_empty());
        assert!(facts.previous_week.iter().any(|s| s.verdict == "missed"));
    }
}

#[cfg(test)]
mod real_database {
    use super::*;
    use crate::db::Db;

    /// `RUNNINGMATE_TEST_DB=/tmp/copy.db cargo test plans_from_a_real_database -- --ignored
    /// --nocapture`; writes a plan into the copy.
    #[test]
    #[ignore = "needs RUNNINGMATE_TEST_DB pointing at a copy of running_data.db"]
    fn plans_from_a_real_database() {
        let path = std::env::var("RUNNINGMATE_TEST_DB").expect("RUNNINGMATE_TEST_DB");
        let db = Db::open(std::path::Path::new(&path)).unwrap();
        let conn = db.conn().unwrap();
        let now = chrono::Utc::now().timestamp();
        println!("{:#?}", current_baseline(&conn, now).unwrap());

        // A 10k plan over 8 weeks that started 6 weeks ago, so past runs get matched.
        let start = today_in(&Local, now) - Days::new(42);
        let start = start - Days::new(u64::from(generator::weekday(start)));
        let input = PlanInput {
            name: "10k test".into(),
            target: TrainingTarget::Race,
            distance: Some(10.0),
            target_time: None,
            start: format_date(start),
            weeks: 8,
            days: vec![1, 3, 6],
            long_day: 6,
        };
        let preview = preview(&conn, &input, now).unwrap();
        println!("warnings {:?}, paces {:?}", preview.warnings, preview.paces);
        let plan = create(&conn, &input, false, now).unwrap();
        for status in detail(&conn, plan.id, now).unwrap().sessions {
            let s = &status.session;
            println!(
                "{} w{} {:?} {:?} {:.1} km {:?} -> {:?} {:?}",
                s.date,
                s.week + 1,
                s.phase,
                s.kind,
                s.distance,
                s.pace
                    .map(|p| (coach::pace_text(p.fast), coach::pace_text(p.slow))),
                status.check.verdict,
                status
                    .run
                    .map(|r| (r.day, r.distance, r.pace.map(coach::pace_text))),
            );
        }
        let adapted = adapt(&conn, plan.id, now).unwrap();
        println!("after adapting: {} sessions", adapted.sessions.len());
        delete(&conn, plan.id).unwrap();
    }
}
