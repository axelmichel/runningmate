use tauri::State;

use crate::{
    db::{
        Db,
        goals::{self, Goal, GoalHit, GoalInput, GoalProgress},
    },
    error::AppResult,
};

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Progress now, or as it stood at `at` (Unix timestamp), for the dashboard looking back.
#[tauri::command(async)]
pub fn goal_progress(
    db: State<'_, Db>,
    include_archived: bool,
    at: Option<i64>,
) -> AppResult<Vec<GoalProgress>> {
    goals::progress(&*db.conn()?, at.unwrap_or_else(now), include_archived)
}

#[tauri::command(async)]
pub fn create_goal(db: State<'_, Db>, input: GoalInput) -> AppResult<Goal> {
    goals::create(&*db.conn()?, &input, now())
}

#[tauri::command(async)]
pub fn update_goal(db: State<'_, Db>, id: i64, input: GoalInput) -> AppResult<Goal> {
    goals::update(&*db.conn()?, id, &input)
}

#[tauri::command(async)]
pub fn archive_goal(db: State<'_, Db>, id: i64, archived: bool) -> AppResult<Goal> {
    goals::set_archived(&*db.conn()?, id, archived)
}

#[tauri::command(async)]
pub fn delete_goal(db: State<'_, Db>, id: i64) -> AppResult<()> {
    goals::delete(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn activity_goal_hits(db: State<'_, Db>, activity_id: i64) -> AppResult<Vec<GoalHit>> {
    goals::activity_hits(&*db.conn()?, now(), activity_id)
}
