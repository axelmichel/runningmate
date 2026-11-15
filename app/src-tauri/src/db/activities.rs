//! Activity list, detail, edit and delete — the Rust counterpart of the `fetch_*`, `update_*`
//! and `delete_activity` methods in `database/database_handler.py`.
//!
//! Values are returned raw (Unix timestamps, seconds, km); formatting happens in the frontend.

use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::value::{bool_or, opt_f64, opt_i64, opt_string};
use crate::{
    activity_type::ViewMode,
    error::{AppError, AppResult},
};

/// Filters from the search bar and filter dialog. Unset fields don't filter.
#[derive(Debug, Default, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct ActivityFilter {
    /// Matched against title and comment.
    pub search_text: Option<String>,
    /// Unix timestamp, inclusive.
    #[ts(type = "number", optional)]
    pub min_date: Option<i64>,
    #[ts(type = "number", optional)]
    pub max_date: Option<i64>,
    /// Kilometres.
    pub min_distance: Option<f64>,
    pub max_distance: Option<f64>,
    /// Seconds.
    #[ts(type = "number", optional)]
    pub min_duration: Option<i64>,
    #[ts(type = "number", optional)]
    pub max_duration: Option<i64>,
    /// Metres.
    pub min_elevation: Option<f64>,
    pub max_elevation: Option<f64>,
}

impl ActivityFilter {
    /// Appends `AND …` conditions for the set fields to `sql` and their values to `params`.
    fn apply(&self, sql: &mut String, params: &mut Vec<Value>) {
        if let Some(text) = self.search_text.as_deref().map(str::trim)
            && !text.is_empty()
        {
            sql.push_str(" AND (activities.title LIKE ? OR activities.comment LIKE ?)");
            let pattern = format!("%{text}%");
            params.push(Value::Text(pattern.clone()));
            params.push(Value::Text(pattern));
        }
        let ranges: [(&str, Option<Value>, Option<Value>); 4] = [
            (
                "activities.date",
                self.min_date.map(Value::Integer),
                self.max_date.map(Value::Integer),
            ),
            (
                "activities.distance",
                self.min_distance.map(Value::Real),
                self.max_distance.map(Value::Real),
            ),
            (
                "activities.duration",
                self.min_duration.map(Value::Integer),
                self.max_duration.map(Value::Integer),
            ),
            (
                "activities.elevation_gain",
                self.min_elevation.map(Value::Real),
                self.max_elevation.map(Value::Real),
            ),
        ];
        for (column, min, max) in ranges {
            if let Some(min) = min {
                sql.push_str(&format!(" AND {column} >= ?"));
                params.push(min);
            }
            if let Some(max) = max {
                sql.push_str(&format!(" AND {column} <= ?"));
                params.push(max);
            }
        }
    }
}

/// Sortable table columns. Type-specific columns fall back to the date in the `All` view.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SortField {
    #[default]
    Date,
    Title,
    Type,
    Duration,
    Distance,
    ElevationGain,
    AvgSpeed,
    AvgPace,
    AvgHeartRate,
    TotalSteps,
    AvgPower,
    Pause,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityQuery {
    pub view_mode: ViewMode,
    #[ts(type = "number")]
    pub offset: i64,
    #[ts(type = "number")]
    pub limit: i64,
    #[serde(default)]
    pub sort_field: SortField,
    #[serde(default)]
    pub sort_direction: SortDirection,
    #[serde(default)]
    pub filter: ActivityFilter,
}

/// One row of the activity table. Type-specific metrics are `null` in the `All` view.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityRow {
    #[ts(type = "number")]
    pub id: i64,
    /// Unix timestamp of the start.
    #[ts(type = "number | null")]
    pub date: Option<i64>,
    pub title: Option<String>,
    pub activity_type: Option<String>,
    /// View the activity type belongs to (`All` for unknown types).
    pub view_mode: ViewMode,
    /// `type_detail` if set, otherwise `activity_type`.
    pub display_type: Option<String>,
    /// Seconds.
    #[ts(type = "number | null")]
    pub duration: Option<i64>,
    /// Kilometres.
    pub distance: Option<f64>,
    /// Metres.
    pub elevation_gain: Option<f64>,
    /// km/h.
    pub avg_speed: Option<f64>,
    /// "MM:SS" per km, as stored by the Python importer.
    pub avg_pace: Option<String>,
    #[ts(type = "number | null")]
    pub avg_heart_rate: Option<i64>,
    #[ts(type = "number | null")]
    pub avg_power: Option<i64>,
    #[ts(type = "number | null")]
    pub avg_steps: Option<i64>,
    #[ts(type = "number | null")]
    pub total_steps: Option<i64>,
    /// "MM:SS".
    pub pause: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityPage {
    pub rows: Vec<ActivityRow>,
    /// Number of rows matching the view and filter, ignoring paging.
    #[ts(type = "number")]
    pub total: i64,
}

/// Totals shown in the info cards above the table.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivitySummary {
    #[ts(type = "number")]
    pub count: i64,
    pub distance: f64,
    #[ts(type = "number")]
    pub duration: i64,
    pub elevation_gain: f64,
}

/// `FROM` clause for a view: the per-type table joined to `activities`, so the list, its count
/// and the summary always cover the same rows.
fn from_clause(view: ViewMode) -> String {
    match view.table() {
        Some(table) => {
            format!(" FROM {table} JOIN activities ON activities.id = {table}.activity_id")
        }
        None => " FROM activities".to_string(),
    }
}

fn type_column(view: ViewMode, column: &str) -> String {
    match view.table() {
        Some(table) => format!("{table}.{column}"),
        None => "NULL".to_string(),
    }
}

fn sort_expression(view: ViewMode, field: SortField) -> String {
    let typed = |column: &str| match view.table() {
        Some(table) => format!("{table}.{column}"),
        None => "activities.date".to_string(),
    };
    match field {
        SortField::Date => "activities.date".into(),
        SortField::Title => "activities.title".into(),
        SortField::Type => {
            "COALESCE(NULLIF(activities.type_detail, ''), activities.activity_type)".into()
        }
        SortField::Duration => "activities.duration".into(),
        SortField::Distance => "activities.distance".into(),
        SortField::ElevationGain => "activities.elevation_gain".into(),
        SortField::AvgSpeed => typed("avg_speed"),
        SortField::AvgPace => typed("avg_pace"),
        SortField::AvgHeartRate => typed("avg_heart_rate"),
        SortField::AvgPower => typed("avg_power"),
        SortField::Pause => typed("pause"),
        // Cycling has no step columns.
        SortField::TotalSteps if view == ViewMode::Cycling => "activities.date".into(),
        SortField::TotalSteps => typed("total_steps"),
    }
}

fn read_row(row: &Row) -> rusqlite::Result<ActivityRow> {
    let activity_type = opt_string(row, "activity_type")?;
    Ok(ActivityRow {
        id: row.get("id")?,
        date: opt_i64(row, "date")?,
        title: opt_string(row, "title")?,
        view_mode: ViewMode::from_activity_type(activity_type.as_deref().unwrap_or_default()),
        activity_type,
        display_type: opt_string(row, "display_type")?,
        duration: opt_i64(row, "duration")?,
        distance: opt_f64(row, "distance")?,
        elevation_gain: opt_f64(row, "elevation_gain")?,
        avg_speed: opt_f64(row, "avg_speed")?,
        avg_pace: opt_string(row, "avg_pace")?,
        avg_heart_rate: opt_i64(row, "avg_heart_rate")?,
        avg_power: opt_i64(row, "avg_power")?,
        avg_steps: opt_i64(row, "avg_steps")?,
        total_steps: opt_i64(row, "total_steps")?,
        pause: opt_string(row, "pause")?,
    })
}

/// A page of the activity table plus the total row count for the pagination.
pub fn list(conn: &Connection, query: &ActivityQuery) -> AppResult<ActivityPage> {
    let view = query.view_mode;
    let steps = |column: &str| {
        if view == ViewMode::Cycling {
            "NULL".to_string()
        } else {
            type_column(view, column)
        }
    };

    let mut sql = format!(
        "SELECT activities.id, activities.date, activities.title, activities.activity_type,
            COALESCE(NULLIF(activities.type_detail, ''), activities.activity_type) AS display_type,
            activities.duration, activities.distance, activities.elevation_gain,
            {} AS avg_speed, {} AS avg_pace, {} AS avg_heart_rate, {} AS avg_power,
            {} AS avg_steps, {} AS total_steps, {} AS pause
         {} WHERE 1=1",
        type_column(view, "avg_speed"),
        type_column(view, "avg_pace"),
        type_column(view, "avg_heart_rate"),
        type_column(view, "avg_power"),
        steps("avg_steps"),
        steps("total_steps"),
        type_column(view, "pause"),
        from_clause(view),
    );
    let mut params = Vec::new();
    query.filter.apply(&mut sql, &mut params);

    let direction = match query.sort_direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    // The id keeps the order stable between pages when sort values are equal.
    sql.push_str(&format!(
        " ORDER BY {} {direction}, activities.id {direction} LIMIT ? OFFSET ?",
        sort_expression(view, query.sort_field)
    ));
    params.push(Value::Integer(query.limit.max(0)));
    params.push(Value::Integer(query.offset.max(0)));

    let rows = conn
        .prepare(&sql)?
        .query_map(params_from_iter(params), read_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ActivityPage {
        rows,
        total: count(conn, view, &query.filter)?,
    })
}

pub fn count(conn: &Connection, view: ViewMode, filter: &ActivityFilter) -> AppResult<i64> {
    let mut sql = format!("SELECT COUNT(*){} WHERE 1=1", from_clause(view));
    let mut params = Vec::new();
    filter.apply(&mut sql, &mut params);
    Ok(conn.query_row(&sql, params_from_iter(params), |row| row.get(0))?)
}

pub fn summary(
    conn: &Connection,
    view: ViewMode,
    filter: &ActivityFilter,
) -> AppResult<ActivitySummary> {
    let mut sql = format!(
        "SELECT COUNT(*) AS count, SUM(activities.distance) AS distance,
            SUM(activities.duration) AS duration, SUM(activities.elevation_gain) AS elevation_gain
         {} WHERE 1=1",
        from_clause(view)
    );
    let mut params = Vec::new();
    filter.apply(&mut sql, &mut params);
    Ok(conn.query_row(&sql, params_from_iter(params), |row| {
        Ok(ActivitySummary {
            count: row.get("count")?,
            distance: opt_f64(row, "distance")?.unwrap_or(0.0),
            duration: opt_i64(row, "duration")?.unwrap_or(0),
            elevation_gain: opt_f64(row, "elevation_gain")?.unwrap_or(0.0),
        })
    })?)
}

/// Training time on one local calendar day, for the activity heatmap.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayTotal {
    /// Local date, "YYYY-MM-DD".
    pub day: String,
    /// Seconds.
    #[ts(type = "number")]
    pub duration: i64,
}

/// Total duration per local day between `start` and `end` (Unix timestamps, inclusive). The
/// filter's own date range is ignored; the caller picks the window from it.
pub fn daily_durations(
    conn: &Connection,
    view: ViewMode,
    filter: &ActivityFilter,
    start: i64,
    end: i64,
) -> AppResult<Vec<DayTotal>> {
    let filter = ActivityFilter {
        min_date: None,
        max_date: None,
        ..filter.clone()
    };
    let mut sql = format!(
        "SELECT date(activities.date, 'unixepoch', 'localtime') AS day,
            SUM(activities.duration) AS duration
         {} WHERE activities.date BETWEEN ? AND ?",
        from_clause(view)
    );
    let mut params = vec![Value::Integer(start), Value::Integer(end)];
    filter.apply(&mut sql, &mut params);
    sql.push_str(" GROUP BY day ORDER BY day");
    let rows = conn
        .prepare(&sql)?
        .query_map(params_from_iter(params), |row| {
            Ok(DayTotal {
                day: row.get("day")?,
                duration: opt_i64(row, "duration")?.unwrap_or(0),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Everything the detail window shows about one activity: the `activities` row, the metrics
/// from its per-type table and the assigned shoe or bike.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityDetail {
    #[ts(type = "number")]
    pub id: i64,
    pub view_mode: ViewMode,
    #[ts(type = "number | null")]
    pub date: Option<i64>,
    pub title: Option<String>,
    pub comment: Option<String>,
    pub activity_type: Option<String>,
    pub type_detail: Option<String>,
    #[ts(type = "number | null")]
    pub duration: Option<i64>,
    pub distance: Option<f64>,
    pub elevation_gain: Option<f64>,
    #[ts(type = "number | null")]
    pub calories: Option<i64>,
    pub file_id: Option<String>,
    /// Imported but not yet reviewed.
    pub is_new: bool,
    pub edited: bool,
    /// Saved on the edit page, or given a comment or media (the Python app never set `edited`,
    /// so older activities only show their review through these). The detail window opens on
    /// the edit page until this is true.
    pub reviewed: bool,
    pub avg_speed: Option<f64>,
    pub avg_pace: Option<String>,
    pub fastest_pace: Option<String>,
    pub slowest_pace: Option<String>,
    #[ts(type = "number | null")]
    pub avg_heart_rate: Option<i64>,
    #[ts(type = "number | null")]
    pub avg_power: Option<i64>,
    #[ts(type = "number | null")]
    pub avg_steps: Option<i64>,
    #[ts(type = "number | null")]
    pub total_steps: Option<i64>,
    pub pause: Option<String>,
    /// Shoe (runs, walks) or bike (rides).
    pub equipment: Option<AssignedEquipment>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AssignedEquipment {
    #[ts(type = "number")]
    pub id: i64,
    pub name: Option<String>,
    pub distance: Option<f64>,
    pub in_use: bool,
}

/// Shoe or bike column and table for a view, `None` for `All`.
fn equipment_table(view: ViewMode) -> Option<(&'static str, &'static str)> {
    match view {
        ViewMode::Running | ViewMode::Walking => Some(("shoe_id", "shoes")),
        ViewMode::Cycling => Some(("bike_id", "bikes")),
        ViewMode::All => None,
    }
}

/// Adds `km` (negative to take it off) to the shoe or bike assigned to an activity, never going
/// below 0. Keeps the item's total in step when the activity's distance changes or it goes.
pub(crate) fn add_to_equipment(conn: &Connection, activity_id: i64, km: f64) -> AppResult<()> {
    let view = ViewMode::from_activity_type(&activity_type_of(conn, activity_id)?);
    let (Some(table), Some((column, equipment))) = (view.table(), equipment_table(view)) else {
        return Ok(());
    };
    if km != 0.0 {
        conn.execute(
            &format!(
                "UPDATE {equipment} SET distance = MAX(COALESCE(distance, 0) + ?1, 0)
                 WHERE id = (SELECT {column} FROM {table} WHERE activity_id = ?2)"
            ),
            params![km, activity_id],
        )?;
    }
    Ok(())
}

/// The activity's stored distance in km, 0 without one.
pub(crate) fn distance_of(conn: &Connection, id: i64) -> AppResult<f64> {
    Ok(conn
        .query_row(
            "SELECT distance FROM activities WHERE id = ?1",
            [id],
            |row| opt_f64(row, "distance"),
        )
        .optional()?
        .flatten()
        .unwrap_or(0.0))
}

fn activity_type_of(conn: &Connection, id: i64) -> AppResult<String> {
    conn.query_row(
        "SELECT activity_type FROM activities WHERE id = ?1",
        [id],
        |row| opt_string(row, "activity_type"),
    )
    .optional()?
    .ok_or_else(|| AppError::NotFound(format!("activity {id}")))
    .map(Option::unwrap_or_default)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<ActivityDetail> {
    let view = ViewMode::from_activity_type(&activity_type_of(conn, id)?);
    let typed = |column: &str| type_column(view, column);
    let steps = |column: &str| {
        if view == ViewMode::Cycling {
            "NULL".to_string()
        } else {
            typed(column)
        }
    };
    let (equipment_join, equipment_cols) = match (view.table(), equipment_table(view)) {
        (Some(table), Some((column, equipment))) => (
            format!(
                " LEFT JOIN {table} ON {table}.activity_id = activities.id
                  LEFT JOIN {equipment} ON {equipment}.id = {table}.{column}"
            ),
            format!(
                "{equipment}.id AS equipment_id, {equipment}.name AS equipment_name,
                 {equipment}.distance AS equipment_distance, {equipment}.status AS equipment_status"
            ),
        ),
        _ => (
            String::new(),
            "NULL AS equipment_id, NULL AS equipment_name, NULL AS equipment_distance,
             NULL AS equipment_status"
                .to_string(),
        ),
    };

    let sql = format!(
        "SELECT activities.*, {} AS avg_speed, {} AS avg_pace, {} AS fastest_pace,
            {} AS slowest_pace, {} AS avg_heart_rate, {} AS avg_power, {} AS avg_steps,
            {} AS total_steps, {} AS pause, {equipment_cols},
            EXISTS (SELECT 1 FROM media WHERE media.activity_id = activities.id) AS has_media
         FROM activities{equipment_join}
         WHERE activities.id = ?1",
        typed("avg_speed"),
        typed("avg_pace"),
        typed("fastest_pace"),
        typed("slowest_pace"),
        typed("avg_heart_rate"),
        typed("avg_power"),
        steps("avg_steps"),
        steps("total_steps"),
        typed("pause"),
    );

    Ok(conn.query_row(&sql, [id], |row| {
        let equipment = match opt_i64(row, "equipment_id")? {
            Some(equipment_id) => Some(AssignedEquipment {
                id: equipment_id,
                name: opt_string(row, "equipment_name")?,
                distance: opt_f64(row, "equipment_distance")?,
                in_use: bool_or(row, "equipment_status", true)?,
            }),
            None => None,
        };
        let edited = bool_or(row, "edited", false)?;
        let comment = opt_string(row, "comment")?;
        let has_media: bool = row.get("has_media")?;
        Ok(ActivityDetail {
            reviewed: edited
                || has_media
                || comment.as_deref().is_some_and(|c| !c.trim().is_empty()),
            id,
            view_mode: view,
            date: opt_i64(row, "date")?,
            title: opt_string(row, "title")?,
            comment,
            activity_type: opt_string(row, "activity_type")?,
            type_detail: opt_string(row, "type_detail")?,
            duration: opt_i64(row, "duration")?,
            distance: opt_f64(row, "distance")?,
            elevation_gain: opt_f64(row, "elevation_gain")?,
            calories: opt_i64(row, "calories")?,
            file_id: opt_string(row, "file_id")?,
            is_new: bool_or(row, "new", false)?,
            edited,
            avg_speed: opt_f64(row, "avg_speed")?,
            avg_pace: opt_string(row, "avg_pace")?,
            fastest_pace: opt_string(row, "fastest_pace")?,
            slowest_pace: opt_string(row, "slowest_pace")?,
            avg_heart_rate: opt_i64(row, "avg_heart_rate")?,
            avg_power: opt_i64(row, "avg_power")?,
            avg_steps: opt_i64(row, "avg_steps")?,
            total_steps: opt_i64(row, "total_steps")?,
            pause: opt_string(row, "pause")?,
            equipment,
        })
    })?)
}

/// Fields the edit page can change. Unlike the Python edit page, which only saved title,
/// comment and type detail, every field shown is saved.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityUpdate {
    #[ts(type = "number")]
    pub id: i64,
    pub title: String,
    pub comment: String,
    pub type_detail: Option<String>,
    #[ts(type = "number")]
    pub date: i64,
    #[ts(type = "number")]
    pub duration: i64,
    pub distance: f64,
    pub elevation_gain: f64,
}

/// Saves the edit page and marks the activity as reviewed (`new = 0`, `edited = 1`). A changed
/// distance is passed on to the assigned shoe or bike.
pub fn update(conn: &mut Connection, update: &ActivityUpdate) -> AppResult<()> {
    if update.duration < 0 || update.distance < 0.0 || update.elevation_gain < 0.0 {
        return Err(AppError::Invalid(
            "duration, distance and elevation must not be negative".into(),
        ));
    }
    let tx = conn.transaction()?;
    let previous = distance_of(&tx, update.id)?;
    let changed = tx.execute(
        "UPDATE activities
         SET title = ?1, comment = ?2, type_detail = ?3, date = ?4, duration = ?5, distance = ?6,
             elevation_gain = ?7, edited = 1, new = 0
         WHERE id = ?8",
        params![
            update.title,
            update.comment,
            update.type_detail,
            update.date,
            update.duration,
            update.distance,
            update.elevation_gain,
            update.id
        ],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("activity {}", update.id)));
    }
    add_to_equipment(&tx, update.id, update.distance - previous)?;
    tx.commit()?;
    Ok(())
}

/// Assigns a shoe or bike (or none) to an activity and moves the activity's distance from the
/// previous one to the new one, as `ui/widget_shoes.py` and `ui/widget_bikes.py` do.
pub fn set_equipment(
    conn: &mut Connection,
    activity_id: i64,
    equipment_id: Option<i64>,
) -> AppResult<()> {
    let view = ViewMode::from_activity_type(&activity_type_of(conn, activity_id)?);
    let (Some(table), Some((column, equipment))) = (view.table(), equipment_table(view)) else {
        return Err(AppError::Invalid(
            "this activity type has no shoes or bikes".into(),
        ));
    };

    let tx = conn.transaction()?;
    let distance: f64 = tx
        .query_row(
            "SELECT distance FROM activities WHERE id = ?1",
            [activity_id],
            |row| opt_f64(row, "distance"),
        )?
        .unwrap_or(0.0);
    let previous: Option<i64> = tx
        .query_row(
            &format!("SELECT {column} FROM {table} WHERE activity_id = ?1"),
            [activity_id],
            |row| opt_i64(row, column),
        )
        .optional()?
        .flatten();

    if previous != equipment_id {
        if let Some(previous) = previous {
            tx.execute(
                &format!(
                    "UPDATE {equipment} SET distance = MAX(COALESCE(distance, 0) - ?1, 0)
                     WHERE id = ?2"
                ),
                params![distance, previous],
            )?;
        }
        if let Some(next) = equipment_id {
            let added = tx.execute(
                &format!(
                    "UPDATE {equipment} SET distance = COALESCE(distance, 0) + ?1 WHERE id = ?2"
                ),
                params![distance, next],
            )?;
            if added == 0 {
                return Err(AppError::NotFound(format!("{equipment} {next}")));
            }
        }
        tx.execute(
            &format!("UPDATE {table} SET {column} = ?1 WHERE activity_id = ?2"),
            params![equipment_id, activity_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Deletes an activity with its per-type row, segments, weather and derived data. Media rows
/// and files are kept, as in the Python app. The assigned shoe or bike gives the distance back.
pub fn delete(conn: &mut Connection, id: i64) -> AppResult<()> {
    let view = ViewMode::from_activity_type(&activity_type_of(conn, id)?);
    let tx = conn.transaction()?;
    add_to_equipment(&tx, id, -distance_of(&tx, id)?)?;
    if let Some(table) = view.table() {
        tx.execute(&format!("DELETE FROM {table} WHERE activity_id = ?1"), [id])?;
    }
    tx.execute("DELETE FROM activities WHERE id = ?1", [id])?;
    for table in [
        "activity_details",
        "weather",
        "best_performances",
        "activity_charts",
        "connector_links",
        "ai_summaries",
        "segment_efforts",
    ] {
        tx.execute(&format!("DELETE FROM {table} WHERE activity_id = ?1"), [id])?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::db::Db;

    /// Inserts an activity plus its per-type row the way the Python importer does.
    pub(crate) fn insert_activity(
        conn: &Connection,
        id: i64,
        activity_type: &str,
        date: i64,
        distance: f64,
        title: &str,
    ) {
        conn.execute(
            "INSERT INTO activities (id, distance, activity_type, comment, title, duration, date,
                elevation_gain, new)
             VALUES (?1, ?2, ?3, '', ?4, 3600, ?5, 50, 1)",
            params![id, distance, activity_type, title, date],
        )
        .unwrap();
        if let Some(table) = ViewMode::from_activity_type(activity_type).table() {
            conn.execute(
                &format!(
                    "INSERT INTO {table} (activity_id, elevation_gain, avg_speed, avg_pace,
                        avg_heart_rate, pause)
                     VALUES (?1, 50, 10.0, '06:00', 140, '00:00')"
                ),
                [id],
            )
            .unwrap();
        }
    }

    fn seeded() -> Db {
        let db = Db::open_in_memory().unwrap();
        {
            let conn = db.conn().unwrap();
            insert_activity(&conn, 1, "Running", 1_700_000_000, 10.0, "Morning run");
            insert_activity(&conn, 2, "Biking", 1_700_100_000, 42.0, "Long ride");
            insert_activity(&conn, 3, "Other", 1_700_200_000, 3.5, "Dog walk");
            insert_activity(&conn, 4, "Trailrun", 1_700_300_000, 21.1, "Trail half");
            conn.execute(
                "UPDATE activities SET comment = 'muddy hills' WHERE id = 4",
                [],
            )
            .unwrap();
        }
        db
    }

    fn query(view_mode: ViewMode) -> ActivityQuery {
        ActivityQuery {
            view_mode,
            offset: 0,
            limit: 25,
            sort_field: SortField::Date,
            sort_direction: SortDirection::Desc,
            filter: ActivityFilter::default(),
        }
    }

    fn ids(page: &ActivityPage) -> Vec<i64> {
        page.rows.iter().map(|r| r.id).collect()
    }

    #[test]
    fn lists_per_view_newest_first() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let all = list(&conn, &query(ViewMode::All)).unwrap();
        assert_eq!(ids(&all), [4, 3, 2, 1]);
        assert_eq!(all.total, 4);
        assert_eq!(all.rows[0].avg_pace, None);

        let runs = list(&conn, &query(ViewMode::Running)).unwrap();
        assert_eq!(ids(&runs), [4, 1]);
        assert_eq!(runs.rows[0].avg_pace.as_deref(), Some("06:00"));

        let rides = list(&conn, &query(ViewMode::Cycling)).unwrap();
        assert_eq!(ids(&rides), [2]);
        assert_eq!(rides.rows[0].total_steps, None);
    }

    #[test]
    fn sorts_and_pages() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut q = query(ViewMode::All);
        q.sort_field = SortField::Distance;
        q.sort_direction = SortDirection::Asc;
        q.limit = 2;
        q.offset = 1;
        let page = list(&conn, &q).unwrap();
        assert_eq!(ids(&page), [1, 4]);
        assert_eq!(page.total, 4);
    }

    #[test]
    fn filters_by_text_and_ranges() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let mut q = query(ViewMode::All);
        q.filter.search_text = Some("MUDDY".into());
        assert_eq!(ids(&list(&conn, &q).unwrap()), [4]);

        let mut q = query(ViewMode::Running);
        q.filter.min_distance = Some(15.0);
        let page = list(&conn, &q).unwrap();
        assert_eq!(ids(&page), [4]);
        assert_eq!(page.total, 1);

        let mut q = query(ViewMode::All);
        q.filter.max_date = Some(1_700_150_000);
        assert_eq!(ids(&list(&conn, &q).unwrap()), [2, 1]);
    }

    #[test]
    fn summarises_a_view() {
        let db = seeded();
        let conn = db.conn().unwrap();
        let s = summary(&conn, ViewMode::Running, &ActivityFilter::default()).unwrap();
        assert_eq!(s.count, 2);
        assert!((s.distance - 31.1).abs() < 1e-9);
        assert_eq!(s.duration, 7200);
        assert_eq!(s.elevation_gain, 100.0);
    }

    #[test]
    fn sums_duration_per_day_ignoring_filter_dates() {
        let db = seeded();
        let conn = db.conn().unwrap();
        // A second activity on the same local day as activity 1.
        insert_activity(&conn, 5, "Running", 1_700_000_600, 5.0, "Second run");
        let filter = ActivityFilter {
            min_date: Some(1_800_000_000),
            ..Default::default()
        };
        let days = daily_durations(
            &conn,
            ViewMode::Running,
            &filter,
            1_699_900_000,
            1_700_250_000,
        )
        .unwrap();
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].duration, 7200);
    }

    #[test]
    fn gets_detail_with_equipment() {
        let db = seeded();
        let mut conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO shoes (id, name, distance, status) VALUES (7, 'Racer', 100, 1)",
            [],
        )
        .unwrap();
        set_equipment(&mut conn, 1, Some(7)).unwrap();

        let detail = get(&conn, 1).unwrap();
        assert_eq!(detail.view_mode, ViewMode::Running);
        assert!(detail.is_new);
        let shoe = detail.equipment.unwrap();
        assert_eq!(shoe.name.as_deref(), Some("Racer"));
        assert_eq!(shoe.distance, Some(110.0));

        // Unassigning gives the distance back.
        set_equipment(&mut conn, 1, None).unwrap();
        let distance: f64 = conn
            .query_row("SELECT distance FROM shoes WHERE id = 7", [], |r| r.get(0))
            .unwrap();
        assert_eq!(distance, 100.0);
        assert!(get(&conn, 1).unwrap().equipment.is_none());
    }

    #[test]
    fn comment_or_media_counts_as_reviewed() {
        let db = seeded();
        let conn = db.conn().unwrap();
        assert!(!get(&conn, 1).unwrap().reviewed);
        assert!(get(&conn, 4).unwrap().reviewed, "activity 4 has a comment");
        crate::db::media::insert(&conn, 2, crate::db::media::MediaType::Image, "/m/a.jpg").unwrap();
        assert!(get(&conn, 2).unwrap().reviewed);
    }

    #[test]
    fn update_saves_all_fields_and_marks_reviewed() {
        let db = seeded();
        let mut conn = db.conn().unwrap();
        update(
            &mut conn,
            &ActivityUpdate {
                id: 1,
                title: "Renamed".into(),
                comment: "felt good".into(),
                type_detail: Some("Interval".into()),
                date: 1_700_000_100,
                duration: 3500,
                distance: 10.2,
                elevation_gain: 60.0,
            },
        )
        .unwrap();
        let detail = get(&conn, 1).unwrap();
        assert_eq!(detail.title.as_deref(), Some("Renamed"));
        assert_eq!(detail.type_detail.as_deref(), Some("Interval"));
        assert_eq!(detail.distance, Some(10.2));
        assert!(!detail.is_new);
        assert!(detail.edited);
        assert!(detail.reviewed);
    }

    #[test]
    fn shoe_distance_follows_edits_and_deletion() {
        let db = seeded();
        let mut conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO shoes (id, name, distance, status) VALUES (7, 'Racer', 100, 1)",
            [],
        )
        .unwrap();
        set_equipment(&mut conn, 1, Some(7)).unwrap();
        let shoe = |conn: &Connection| -> f64 {
            conn.query_row("SELECT distance FROM shoes WHERE id = 7", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(shoe(&conn), 110.0);

        let mut edit = ActivityUpdate {
            id: 1,
            title: "Run".into(),
            comment: String::new(),
            type_detail: None,
            date: 1_700_000_000,
            duration: 3600,
            distance: 12.5,
            elevation_gain: 0.0,
        };
        update(&mut conn, &edit).unwrap();
        assert!((shoe(&conn) - 112.5).abs() < 1e-9);
        edit.distance = 9.0;
        update(&mut conn, &edit).unwrap();
        assert!((shoe(&conn) - 109.0).abs() < 1e-9);

        delete(&mut conn, 1).unwrap();
        assert!((shoe(&conn) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn delete_removes_activity_and_related_rows() {
        let db = seeded();
        let mut conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO weather (activity_id, max_temp) VALUES (1, 10)",
            [],
        )
        .unwrap();
        delete(&mut conn, 1).unwrap();
        assert!(matches!(get(&conn, 1), Err(AppError::NotFound(_))));
        let left: i64 = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM runs WHERE activity_id = 1)
                      + (SELECT COUNT(*) FROM weather WHERE activity_id = 1)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(left, 0);
    }
}
