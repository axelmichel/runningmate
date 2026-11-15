//! Goals set by the user (`goals`): a distance, time, elevation or number of activities, reached
//! by one activity or as the total over a week, month, year, a number of days or an open stretch,
//! once or repeating. Progress and the activities that hit a goal are computed on read, so
//! importing, editing or deleting an activity leaves nothing to update.

use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, TimeZone};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    activity_type::ViewMode,
    db::route_segments::{parse_view, view_name},
    error::{AppError, AppResult},
};

/// Earlier windows sent along with the current one.
const HISTORY_SHOWN: usize = 12;
/// Longest window of a `Days` goal, about ten years.
const MAX_DAYS: u32 = 3650;
/// Stored distances have two decimals; a target of 21.1 km is reached by 21.1 km.
const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GoalMetric {
    /// km.
    Distance,
    /// s, the elapsed time of the activities.
    Duration,
    /// m climbed.
    Elevation,
    /// Number of activities.
    Count,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GoalPeriod {
    /// From the start date to the end date, or for good without one.
    Open,
    /// Monday to Sunday.
    Week,
    /// Calendar month.
    Month,
    /// Calendar year.
    Year,
    /// `days` days from the start date.
    Days,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalSpec {
    pub metric: GoalMetric,
    /// km, s, m or activities, following `metric`.
    pub target: f64,
    /// One activity has to reach the target; otherwise the total over the window counts.
    pub per_activity: bool,
    /// s, the most that activity may take, e.g. 10 km in 50 min. Per-activity goals only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max_duration: Option<u32>,
    pub period: GoalPeriod,
    /// Window length of a `Days` goal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub days: Option<u32>,
    /// Starts over in the next window, until the end date if there is one. Week, month and year
    /// windows follow the calendar, starting with the one holding the start date.
    pub repeat: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Goal {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub view: ViewMode,
    pub spec: GoalSpec,
    /// "YYYY-MM-DD", local time.
    pub start: String,
    /// "YYYY-MM-DD", the last day counted.
    pub end: Option<String>,
    pub archived: bool,
    /// Unix timestamp.
    #[ts(type = "number")]
    pub created: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalInput {
    pub name: String,
    pub view: ViewMode,
    pub spec: GoalSpec,
    pub start: String,
    pub end: Option<String>,
}

/// One window of a goal with what was done in it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalWindow {
    /// Unix timestamp.
    #[ts(type = "number")]
    pub start: i64,
    /// Unix timestamp, exclusive; `None` for an open goal without end date.
    #[ts(type = "number | null")]
    pub end: Option<i64>,
    /// The total, or for per-activity goals the best activity within the time limit.
    pub value: f64,
    /// Activities of the goal's sport in the window.
    #[ts(type = "number")]
    pub activities: i64,
    /// Start of the activity that reached the target.
    #[ts(type = "number | null")]
    pub achieved_at: Option<i64>,
    #[ts(type = "number | null")]
    pub achieved_activity_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GoalStatus {
    /// The start date is still ahead.
    Upcoming,
    Active,
    /// A one-time goal that was reached.
    Achieved,
    /// A one-time goal whose window is over without reaching it.
    Missed,
    /// A repeating goal past its end date.
    Ended,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalActivity {
    #[ts(type = "number")]
    pub id: i64,
    pub title: Option<String>,
    /// Unix timestamp.
    #[ts(type = "number")]
    pub date: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalProgress {
    pub goal: Goal,
    pub status: GoalStatus,
    /// The running window, or the last one once the goal is over.
    pub current: GoalWindow,
    /// Earlier windows, newest first, at most twelve.
    pub history: Vec<GoalWindow>,
    /// Windows in which the target was reached.
    #[ts(type = "number")]
    pub achieved: i64,
    /// Windows that are over or reached.
    #[ts(type = "number")]
    pub windows: i64,
    /// Reached windows in a row up to now; a running window not reached yet doesn't break it.
    #[ts(type = "number")]
    pub streak: i64,
    /// Activities that hit the goal.
    #[ts(type = "number")]
    pub hits: i64,
    pub last_hit: Option<GoalActivity>,
}

/// A goal an activity hit.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoalHit {
    #[ts(type = "number")]
    pub goal_id: i64,
    pub name: String,
}

/// What `goals.parameters` holds.
#[derive(Serialize, Deserialize)]
struct Parameters {
    #[serde(flatten)]
    spec: GoalSpec,
    start: String,
    end: Option<String>,
}

fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::Invalid(format!("date must be YYYY-MM-DD: {value}")))
}

/// Checks the input and returns its start and end dates.
fn validate(input: &GoalInput) -> AppResult<(NaiveDate, Option<NaiveDate>)> {
    let spec = &input.spec;
    let invalid = |message: &str| Err(AppError::Invalid(message.into()));
    if input.name.trim().is_empty() {
        return invalid("a goal needs a name");
    }
    if !(spec.target.is_finite() && spec.target > 0.0) {
        return invalid("the target must be above zero");
    }
    if spec.per_activity && spec.metric == GoalMetric::Count {
        return invalid("a number of activities can't be reached by one activity");
    }
    if spec.max_duration.is_some() && (!spec.per_activity || spec.metric == GoalMetric::Duration) {
        return invalid("a time limit only goes with a distance or climb in one activity");
    }
    if spec.max_duration == Some(0) {
        return invalid("the time limit must be above zero");
    }
    if spec.period == GoalPeriod::Days && !spec.days.is_some_and(|d| (1..=MAX_DAYS).contains(&d)) {
        return invalid("the number of days must be between 1 and 3650");
    }
    if spec.period == GoalPeriod::Open && spec.repeat {
        return invalid("a goal without a period can't repeat");
    }
    let start = parse_date(&input.start)?;
    let end = input.end.as_deref().map(parse_date).transpose()?;
    if end.is_some_and(|end| end < start) {
        return invalid("the end date must not be before the start date");
    }
    Ok((start, end))
}

fn goal_from_row(row: &Row) -> rusqlite::Result<Option<Goal>> {
    let parameters: String = row.get("parameters")?;
    // Rows written by anything else than this module are left out.
    let Ok(parameters) = serde_json::from_str::<Parameters>(&parameters) else {
        return Ok(None);
    };
    Ok(Some(Goal {
        id: row.get("id")?,
        name: row.get("name")?,
        view: parse_view(
            &row.get::<_, Option<String>>("activity_types")?
                .unwrap_or_default(),
        ),
        spec: parameters.spec,
        start: parameters.start,
        end: parameters.end,
        archived: row.get::<_, Option<bool>>("archived")?.unwrap_or(false),
        created: row.get::<_, Option<i64>>("created")?.unwrap_or(0),
    }))
}

const GOAL_COLUMNS: &str = "id, name, activity_types, parameters, archived, created";

pub fn list(conn: &Connection) -> AppResult<Vec<Goal>> {
    let mut stmt = conn.prepare(&format!("SELECT {GOAL_COLUMNS} FROM goals ORDER BY id"))?;
    let goals = stmt
        .query_map([], goal_from_row)?
        .filter_map(Result::transpose)
        .collect::<rusqlite::Result<_>>()?;
    Ok(goals)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Goal> {
    conn.query_row(
        &format!("SELECT {GOAL_COLUMNS} FROM goals WHERE id = ?1"),
        [id],
        goal_from_row,
    )
    .optional()?
    .flatten()
    .ok_or_else(|| AppError::NotFound(format!("goal {id}")))
}

fn stored(input: &GoalInput) -> AppResult<(&'static str, String, String)> {
    validate(input)?;
    let goal_type = if input.spec.per_activity {
        "activity"
    } else {
        "total"
    };
    let parameters = serde_json::to_string(&Parameters {
        spec: input.spec.clone(),
        start: input.start.clone(),
        end: input.end.clone(),
    })?;
    Ok((goal_type, view_name(input.view), parameters))
}

pub fn create(conn: &Connection, input: &GoalInput, now: i64) -> AppResult<Goal> {
    let (goal_type, view, parameters) = stored(input)?;
    conn.execute(
        "INSERT INTO goals (name, goal_type, activity_types, parameters, archived, created)
         VALUES (?1, ?2, ?3, ?4, 0, ?5)",
        params![input.name.trim(), goal_type, view, parameters, now],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, input: &GoalInput) -> AppResult<Goal> {
    let (goal_type, view, parameters) = stored(input)?;
    let changed = conn.execute(
        "UPDATE goals SET name = ?2, goal_type = ?3, activity_types = ?4, parameters = ?5
         WHERE id = ?1",
        params![id, input.name.trim(), goal_type, view, parameters],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("goal {id}")));
    }
    get(conn, id)
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> AppResult<Goal> {
    conn.execute(
        "UPDATE goals SET archived = ?2 WHERE id = ?1",
        params![id, archived],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM goals WHERE id = ?1", [id])?;
    Ok(())
}

/// The activity fields goals look at.
struct Activity {
    id: i64,
    view: ViewMode,
    date: i64,
    distance: f64,
    duration: i64,
    elevation: f64,
    title: Option<String>,
}

impl Activity {
    fn value(&self, metric: GoalMetric) -> f64 {
        match metric {
            GoalMetric::Distance => self.distance,
            GoalMetric::Duration => self.duration as f64,
            GoalMetric::Elevation => self.elevation,
            GoalMetric::Count => 1.0,
        }
    }
}

/// Every activity, oldest first.
fn load_activities(conn: &Connection) -> AppResult<Vec<Activity>> {
    let mut stmt = conn.prepare(
        "SELECT id, activity_type, date, distance, duration, elevation_gain, title
         FROM activities WHERE date IS NOT NULL ORDER BY date, id",
    )?;
    let activities = stmt
        .query_map([], |row| {
            Ok(Activity {
                id: row.get(0)?,
                view: ViewMode::from_activity_type(
                    &row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                ),
                date: row.get(2)?,
                distance: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                duration: row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                elevation: row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                title: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(activities)
}

/// Unix timestamp of 00:00 on `day` in `tz`.
pub(crate) fn midnight<Tz: TimeZone>(tz: &Tz, day: NaiveDate) -> i64 {
    let time = day.and_hms_opt(0, 0, 0).unwrap_or_default();
    tz.from_local_datetime(&time)
        .earliest()
        .map_or_else(|| time.and_utc().timestamp(), |start| start.timestamp())
}

/// First day of the window holding `day`.
fn window_start(period: GoalPeriod, day: NaiveDate) -> NaiveDate {
    match period {
        GoalPeriod::Week => day - Days::new(u64::from(day.weekday().num_days_from_monday())),
        GoalPeriod::Month => day.with_day(1).unwrap_or(day),
        GoalPeriod::Year => NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap_or(day),
        GoalPeriod::Open | GoalPeriod::Days => day,
    }
}

/// First day after the window starting on `start`; `None` for an open window.
fn window_after(spec: &GoalSpec, start: NaiveDate) -> Option<NaiveDate> {
    match spec.period {
        GoalPeriod::Open => None,
        GoalPeriod::Week => start.checked_add_days(Days::new(7)),
        GoalPeriod::Month => start.checked_add_months(Months::new(1)),
        GoalPeriod::Year => start.checked_add_months(Months::new(12)),
        GoalPeriod::Days => start.checked_add_days(Days::new(u64::from(spec.days.unwrap_or(1)))),
    }
}

/// The windows of a goal as days, `[first, after)`, up to the one holding `today` or the end
/// date, whichever comes first.
fn window_days(
    spec: &GoalSpec,
    start: NaiveDate,
    end: Option<NaiveDate>,
    today: NaiveDate,
) -> Vec<(NaiveDate, Option<NaiveDate>)> {
    if spec.period == GoalPeriod::Open {
        return vec![(start, end.and_then(|end| end.succ_opt()))];
    }
    let last = end.map_or(today, |end| end.min(today));
    let mut windows = Vec::new();
    let mut first = window_start(spec.period, start);
    loop {
        let after = window_after(spec, first);
        windows.push((first, after));
        match after {
            Some(next) if spec.repeat && next <= last => first = next,
            _ => break,
        }
    }
    windows
}

struct Evaluation {
    windows: Vec<GoalWindow>,
    /// Indices of the activities that hit the goal, oldest first.
    hits: Vec<usize>,
}

fn evaluate<Tz: TimeZone>(
    goal: &Goal,
    activities: &[Activity],
    tz: &Tz,
    today: NaiveDate,
) -> AppResult<Evaluation> {
    let spec = &goal.spec;
    let start = parse_date(&goal.start)?;
    let end = goal.end.as_deref().map(parse_date).transpose()?;
    let mut hits = Vec::new();
    let windows = window_days(spec, start, end, today)
        .into_iter()
        .map(|(first, after)| {
            let start = midnight(tz, first);
            let end = after.map(|after| midnight(tz, after));
            let from = activities.partition_point(|a| a.date < start);
            let to = end.map_or(activities.len(), |end| {
                activities.partition_point(|a| a.date < end)
            });
            let mut window = GoalWindow {
                start,
                end,
                value: 0.0,
                activities: 0,
                achieved_at: None,
                achieved_activity_id: None,
            };
            for (index, activity) in activities[from..to]
                .iter()
                .enumerate()
                .map(|(i, a)| (from + i, a))
                .filter(|(_, a)| goal.view == ViewMode::All || a.view == goal.view)
            {
                window.activities += 1;
                let value = activity.value(spec.metric);
                let hit = if spec.per_activity {
                    if spec
                        .max_duration
                        .is_some_and(|limit| activity.duration > i64::from(limit))
                    {
                        continue;
                    }
                    window.value = window.value.max(value);
                    value + EPSILON >= spec.target
                } else {
                    window.value += value;
                    window.achieved_at.is_none() && window.value + EPSILON >= spec.target
                };
                if hit {
                    hits.push(index);
                    if window.achieved_at.is_none() {
                        window.achieved_at = Some(activity.date);
                        window.achieved_activity_id = Some(activity.id);
                    }
                }
            }
            window
        })
        .collect();
    Ok(Evaluation { windows, hits })
}

pub(crate) fn today_in<Tz: TimeZone>(tz: &Tz, now: i64) -> NaiveDate {
    DateTime::from_timestamp(now, 0)
        .unwrap_or_default()
        .with_timezone(tz)
        .date_naive()
}

fn progress_of<Tz: TimeZone>(
    goal: Goal,
    activities: &[Activity],
    tz: &Tz,
    now: i64,
) -> AppResult<GoalProgress> {
    let Evaluation { mut windows, hits } = evaluate(&goal, activities, tz, today_in(tz, now))?;
    let is_over = |window: &GoalWindow| window.end.is_some_and(|end| end <= now);
    let reached = |window: &GoalWindow| window.achieved_at.is_some();
    let achieved = windows.iter().filter(|w| reached(w)).count() as i64;
    let finished = windows.iter().filter(|w| reached(w) || is_over(w)).count() as i64;
    let skip_running = usize::from(windows.last().is_some_and(|w| !reached(w) && !is_over(w)));
    let streak = windows
        .iter()
        .rev()
        .skip(skip_running)
        .take_while(|w| reached(w))
        .count() as i64;

    let start = midnight(tz, parse_date(&goal.start)?);
    let end = goal
        .end
        .as_deref()
        .map(parse_date)
        .transpose()?
        .and_then(|end| end.succ_opt())
        .map(|after| midnight(tz, after));
    // Only one window when the goal doesn't repeat, and windows are made up to today.
    let current = windows
        .pop()
        .ok_or_else(|| AppError::Invalid("goal without window".into()))?;
    let status = if !goal.spec.repeat && reached(&current) {
        GoalStatus::Achieved
    } else if now < start {
        GoalStatus::Upcoming
    } else if !goal.spec.repeat && is_over(&current) {
        GoalStatus::Missed
    } else if goal.spec.repeat && end.is_some_and(|end| end <= now) {
        GoalStatus::Ended
    } else {
        GoalStatus::Active
    };
    let last_hit = hits.last().map(|&index| {
        let activity = &activities[index];
        GoalActivity {
            id: activity.id,
            title: activity.title.clone(),
            date: activity.date,
        }
    });
    Ok(GoalProgress {
        goal,
        status,
        current,
        history: windows.into_iter().rev().take(HISTORY_SHOWN).collect(),
        achieved,
        windows: finished,
        streak,
        hits: hits.len() as i64,
        last_hit,
    })
}

fn progress_in<Tz: TimeZone>(
    conn: &Connection,
    tz: &Tz,
    now: i64,
    include_archived: bool,
) -> AppResult<Vec<GoalProgress>> {
    // Looking back from an earlier moment, later activities don't count yet.
    let mut activities = load_activities(conn)?;
    activities.retain(|a| a.date <= now);
    list(conn)?
        .into_iter()
        .filter(|goal| include_archived || !goal.archived)
        .map(|goal| progress_of(goal, &activities, tz, now))
        .collect()
}

/// Progress of every goal, oldest goal first, as it stood at `now` in local time.
pub fn progress(
    conn: &Connection,
    now: i64,
    include_archived: bool,
) -> AppResult<Vec<GoalProgress>> {
    progress_in(conn, &Local, now, include_archived)
}

fn activity_hits_in<Tz: TimeZone>(
    conn: &Connection,
    tz: &Tz,
    now: i64,
    activity_id: i64,
) -> AppResult<Vec<GoalHit>> {
    let activities = load_activities(conn)?;
    let Some(index) = activities.iter().position(|a| a.id == activity_id) else {
        return Ok(Vec::new());
    };
    let today = today_in(tz, now);
    let mut hit = Vec::new();
    for goal in list(conn)? {
        if goal.view != ViewMode::All && goal.view != activities[index].view {
            continue;
        }
        if evaluate(&goal, &activities, tz, today)?
            .hits
            .contains(&index)
        {
            hit.push(GoalHit {
                goal_id: goal.id,
                name: goal.name,
            });
        }
    }
    Ok(hit)
}

/// Goals the activity hit, archived ones included.
pub fn activity_hits(conn: &Connection, now: i64, activity_id: i64) -> AppResult<Vec<GoalHit>> {
    activity_hits_in(conn, &Local, now, activity_id)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    /// Unix timestamp of `day` at `hour`:00 UTC.
    fn at(day: &str, hour: u32) -> i64 {
        parse_date(day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    fn spec(metric: GoalMetric, target: f64, per_activity: bool, period: GoalPeriod) -> GoalSpec {
        GoalSpec {
            metric,
            target,
            per_activity,
            max_duration: None,
            period,
            days: None,
            repeat: false,
        }
    }

    fn input(view: ViewMode, spec: GoalSpec, start: &str) -> GoalInput {
        GoalInput {
            name: "Goal".into(),
            view,
            spec,
            start: start.into(),
            end: None,
        }
    }

    fn set_duration(conn: &Connection, id: i64, duration: i64) {
        conn.execute(
            "UPDATE activities SET duration = ?2 WHERE id = ?1",
            params![id, duration],
        )
        .unwrap();
    }

    /// Runs on Mon 3, Wed 5 and Sun 9 November 2025, a ride on Tue 4 and a run on Mon 10.
    fn seeded() -> Db {
        let db = Db::open_in_memory().unwrap();
        {
            let conn = db.conn().unwrap();
            insert_activity(&conn, 1, "Running", at("2025-11-03", 7), 5.0, "Easy");
            insert_activity(&conn, 2, "Biking", at("2025-11-04", 17), 40.0, "Ride");
            insert_activity(&conn, 3, "Running", at("2025-11-05", 7), 10.0, "Tempo");
            insert_activity(&conn, 4, "Running", at("2025-11-09", 9), 21.1, "Long");
            insert_activity(&conn, 5, "Running", at("2025-11-10", 7), 8.0, "Monday");
            set_duration(&conn, 3, 2900);
            set_duration(&conn, 4, 7200);
        }
        db
    }

    fn progress_at(conn: &Connection, now: i64) -> Vec<GoalProgress> {
        progress_in(conn, &Utc, now, true).unwrap()
    }

    fn hits(conn: &Connection, now: i64, activity_id: i64) -> Vec<i64> {
        activity_hits_in(conn, &Utc, now, activity_id)
            .unwrap()
            .into_iter()
            .map(|hit| hit.goal_id)
            .collect()
    }

    #[test]
    fn windows_follow_the_calendar() {
        let day = |s| parse_date(s).unwrap();
        let mut weekly = spec(GoalMetric::Count, 3.0, false, GoalPeriod::Week);
        weekly.repeat = true;
        assert_eq!(
            window_days(&weekly, day("2025-11-05"), None, day("2025-11-10")),
            [
                (day("2025-11-03"), Some(day("2025-11-10"))),
                (day("2025-11-10"), Some(day("2025-11-17")))
            ]
        );
        let monthly = spec(GoalMetric::Distance, 100.0, false, GoalPeriod::Month);
        assert_eq!(
            window_days(&monthly, day("2025-01-31"), None, day("2025-06-01")),
            [(day("2025-01-01"), Some(day("2025-02-01")))]
        );
        let yearly = spec(GoalMetric::Distance, 100.0, false, GoalPeriod::Year);
        assert_eq!(
            window_days(&yearly, day("2025-06-15"), None, day("2025-06-15")),
            [(day("2025-01-01"), Some(day("2026-01-01")))]
        );
        let mut every_ten = spec(GoalMetric::Distance, 50.0, false, GoalPeriod::Days);
        every_ten.days = Some(10);
        every_ten.repeat = true;
        assert_eq!(
            window_days(
                &every_ten,
                day("2025-11-01"),
                Some(day("2025-11-12")),
                day("2026-01-01")
            ),
            [
                (day("2025-11-01"), Some(day("2025-11-11"))),
                (day("2025-11-11"), Some(day("2025-11-21")))
            ]
        );
        let open = spec(GoalMetric::Distance, 500.0, false, GoalPeriod::Open);
        assert_eq!(
            window_days(
                &open,
                day("2025-11-01"),
                Some(day("2025-11-30")),
                day("2025-11-05")
            ),
            [(day("2025-11-01"), Some(day("2025-12-01")))]
        );
    }

    #[test]
    fn a_week_window_starts_on_monday_in_local_time() {
        let tz = chrono::FixedOffset::east_opt(2 * 3600).unwrap();
        let goal = Goal {
            id: 1,
            name: "Goal".into(),
            view: ViewMode::All,
            spec: spec(GoalMetric::Count, 1.0, false, GoalPeriod::Week),
            start: "2025-11-05".into(),
            end: None,
            archived: false,
            created: 0,
        };
        let evaluation = evaluate(&goal, &[], &tz, parse_date("2025-11-05").unwrap()).unwrap();
        assert_eq!(evaluation.windows[0].start, at("2025-11-02", 22));
    }

    #[test]
    fn the_activity_crossing_a_total_gets_the_star() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut weekly = spec(GoalMetric::Count, 3.0, false, GoalPeriod::Week);
        weekly.repeat = true;
        let goal = create(&conn, &input(ViewMode::Running, weekly, "2025-11-03"), 0).unwrap();

        let now = at("2025-11-12", 12);
        let [progress] = progress_at(&conn, now).try_into().unwrap();
        assert_eq!(progress.status, GoalStatus::Active);
        assert_eq!(progress.current.value, 1.0);
        assert_eq!(progress.current.achieved_at, None);
        assert_eq!(progress.history.len(), 1);
        assert_eq!(progress.history[0].value, 3.0);
        assert_eq!(progress.history[0].achieved_activity_id, Some(4));
        assert_eq!(
            (progress.achieved, progress.windows, progress.streak),
            (1, 1, 1)
        );
        assert_eq!(progress.hits, 1);
        assert_eq!(progress.last_hit.map(|hit| hit.id), Some(4));

        assert_eq!(hits(&conn, now, 4), [goal.id]);
        assert!(hits(&conn, now, 3).is_empty());
        // The ride doesn't count for a running goal.
        assert!(hits(&conn, now, 2).is_empty());
    }

    #[test]
    fn progress_at_an_earlier_moment_leaves_out_later_activities() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut weekly = spec(GoalMetric::Count, 3.0, false, GoalPeriod::Week);
        weekly.repeat = true;
        create(&conn, &input(ViewMode::Running, weekly, "2025-11-03"), 0).unwrap();

        // Saturday 8 November: two runs so far, the long run on Sunday is still to come.
        let [progress] = progress_at(&conn, at("2025-11-08", 23)).try_into().unwrap();
        assert_eq!(progress.current.value, 2.0);
        assert_eq!(progress.current.achieved_at, None);
        assert_eq!(progress.hits, 0);
    }

    #[test]
    fn totals_add_up_distance_time_and_climb() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let month = |metric, target| spec(metric, target, false, GoalPeriod::Month);
        create(
            &conn,
            &input(
                ViewMode::All,
                month(GoalMetric::Distance, 80.0),
                "2025-11-01",
            ),
            0,
        )
        .unwrap();
        create(
            &conn,
            &input(
                ViewMode::Running,
                month(GoalMetric::Duration, 20000.0),
                "2025-11-01",
            ),
            0,
        )
        .unwrap();
        create(
            &conn,
            &input(
                ViewMode::Cycling,
                month(GoalMetric::Elevation, 100.0),
                "2025-11-01",
            ),
            0,
        )
        .unwrap();

        let progress = progress_at(&conn, at("2025-11-20", 12));
        assert!((progress[0].current.value - 84.1).abs() < 1e-9);
        assert_eq!(progress[0].current.achieved_activity_id, Some(5));
        assert_eq!(progress[0].status, GoalStatus::Achieved);
        assert_eq!(
            progress[1].current.value,
            (3600 + 2900 + 7200 + 3600) as f64
        );
        assert_eq!(progress[1].status, GoalStatus::Active);
        assert_eq!(progress[2].current.value, 50.0);
        assert_eq!(progress[2].current.activities, 1);

        // December: the one-time November goals are over.
        let progress = progress_at(&conn, at("2025-12-02", 12));
        assert_eq!(progress[0].status, GoalStatus::Achieved);
        assert_eq!(progress[1].status, GoalStatus::Missed);
    }

    #[test]
    fn one_activity_has_to_reach_distance_within_the_time_limit() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut ten_k = spec(GoalMetric::Distance, 10.0, true, GoalPeriod::Open);
        ten_k.max_duration = Some(3000);
        let fast = create(&conn, &input(ViewMode::Running, ten_k, "2025-11-01"), 0).unwrap();
        let half = spec(GoalMetric::Distance, 21.1, true, GoalPeriod::Open);
        let long = create(&conn, &input(ViewMode::Running, half, "2025-11-01"), 0).unwrap();

        let now = at("2025-11-20", 12);
        // 10 km in 2900 s qualifies, 21.1 km in 7200 s is over the limit.
        assert_eq!(hits(&conn, now, 3), [fast.id]);
        assert_eq!(hits(&conn, now, 4), [long.id]);
        let progress = progress_at(&conn, now);
        assert_eq!(progress[0].current.value, 10.0);
        assert_eq!(progress[0].current.end, None);
        assert_eq!(progress[0].status, GoalStatus::Achieved);
        assert_eq!(progress[1].hits, 1);
    }

    #[test]
    fn a_repeating_per_activity_goal_stars_every_qualifying_activity() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut weekly = spec(GoalMetric::Distance, 8.0, true, GoalPeriod::Week);
        weekly.repeat = true;
        let goal = create(&conn, &input(ViewMode::Running, weekly, "2025-11-03"), 0).unwrap();

        let now = at("2025-11-25", 12);
        for id in [3, 4, 5] {
            assert_eq!(hits(&conn, now, id), [goal.id], "activity {id}");
        }
        let [progress] = progress_at(&conn, now).try_into().unwrap();
        assert_eq!(progress.hits, 3);
        // The weeks of 3 and 10 November are reached, the week of 17 had no run and the week
        // of 24 is still running.
        assert_eq!((progress.achieved, progress.windows), (2, 3));
        assert_eq!(progress.streak, 0);
    }

    #[test]
    fn nothing_before_the_start_or_after_the_end_counts() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut window = input(
            ViewMode::Running,
            spec(GoalMetric::Distance, 1.0, true, GoalPeriod::Open),
            "2025-11-04",
        );
        window.end = Some("2025-11-08".into());
        create(&conn, &window, 0).unwrap();

        let now = at("2025-11-20", 12);
        assert!(hits(&conn, now, 1).is_empty());
        assert_eq!(hits(&conn, now, 3).len(), 1);
        assert!(hits(&conn, now, 4).is_empty());

        let upcoming = input(
            ViewMode::All,
            spec(GoalMetric::Count, 1.0, false, GoalPeriod::Open),
            "2025-12-01",
        );
        create(&conn, &upcoming, 0).unwrap();
        assert_eq!(progress_at(&conn, now)[1].status, GoalStatus::Upcoming);
    }

    #[test]
    fn a_repeating_goal_ends_on_its_end_date() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut weekly = spec(GoalMetric::Count, 1.0, false, GoalPeriod::Week);
        weekly.repeat = true;
        let mut goal = input(ViewMode::All, weekly, "2025-11-03");
        goal.end = Some("2025-11-09".into());
        create(&conn, &goal, 0).unwrap();

        let [progress] = progress_at(&conn, at("2025-12-01", 12)).try_into().unwrap();
        assert_eq!(progress.status, GoalStatus::Ended);
        assert!(progress.history.is_empty());
        assert_eq!(progress.streak, 1);
    }

    #[test]
    fn invalid_goals_are_refused() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let refused = |input: GoalInput| {
            assert!(matches!(
                create(&conn, &input, 0),
                Err(AppError::Invalid(_))
            ));
        };
        let total = spec(GoalMetric::Distance, 100.0, false, GoalPeriod::Month);
        let base = input(ViewMode::All, total.clone(), "2025-11-01");

        refused(GoalInput {
            name: " ".into(),
            ..base.clone()
        });
        refused(GoalInput {
            start: "1.11.2025".into(),
            ..base.clone()
        });
        refused(GoalInput {
            end: Some("2025-10-01".into()),
            ..base.clone()
        });
        refused(GoalInput {
            spec: GoalSpec {
                target: 0.0,
                ..total.clone()
            },
            ..base.clone()
        });
        let count = spec(GoalMetric::Count, 3.0, true, GoalPeriod::Week);
        refused(GoalInput {
            spec: count,
            ..base.clone()
        });
        let limit = GoalSpec {
            max_duration: Some(3000),
            ..total.clone()
        };
        refused(GoalInput {
            spec: limit,
            ..base.clone()
        });
        let days = GoalSpec {
            period: GoalPeriod::Days,
            ..total.clone()
        };
        refused(GoalInput {
            spec: days,
            ..base.clone()
        });
        let open = GoalSpec {
            period: GoalPeriod::Open,
            repeat: true,
            ..total
        };
        refused(GoalInput { spec: open, ..base });
    }

    #[test]
    fn goals_are_stored_updated_archived_and_deleted() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let mut every_week = spec(GoalMetric::Distance, 30.0, false, GoalPeriod::Days);
        every_week.days = Some(7);
        every_week.repeat = true;
        let goal = create(
            &conn,
            &input(ViewMode::Cycling, every_week.clone(), "2025-11-01"),
            42,
        )
        .unwrap();
        assert_eq!(goal.view, ViewMode::Cycling);
        assert_eq!(goal.spec, every_week);
        assert_eq!(goal.created, 42);
        assert!(!goal.archived);

        let mut changed = input(ViewMode::Running, every_week, "2025-11-02");
        changed.name = "  Weekly run ".into();
        let goal = update(&conn, goal.id, &changed).unwrap();
        assert_eq!(
            (goal.name.as_str(), goal.start.as_str()),
            ("Weekly run", "2025-11-02")
        );

        assert!(set_archived(&conn, goal.id, true).unwrap().archived);
        assert!(progress_in(&conn, &Utc, 0, false).unwrap().is_empty());
        assert_eq!(progress_in(&conn, &Utc, 0, true).unwrap().len(), 1);

        delete(&conn, goal.id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
        assert!(matches!(
            update(&conn, goal.id, &changed),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn rows_from_elsewhere_are_left_out() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO goals (name, goal_type, activity_types, parameters)
             VALUES ('Old', 'distance', 'run', '{\"km\": 5}')",
            [],
        )
        .unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }
}
