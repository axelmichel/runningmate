use tauri::State;

use crate::{
    db::{
        Db,
        training::{
            self, PlanDetail, PlanSession, PlanSummary, SessionInput, TrainingPlan, TrainingSession,
        },
    },
    error::AppResult,
    training::{
        baseline::Baseline,
        generator::{PlanInput, Preview},
        matching::MatchedRun,
    },
};

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[tauri::command(async)]
pub fn training_baseline(db: State<'_, Db>) -> AppResult<Baseline> {
    training::current_baseline(&*db.conn()?, now())
}

#[tauri::command(async)]
pub fn preview_training_plan(db: State<'_, Db>, input: PlanInput) -> AppResult<Preview> {
    training::preview(&*db.conn()?, &input, now())
}

#[tauri::command(async)]
pub fn create_training_plan(
    db: State<'_, Db>,
    input: PlanInput,
    create_goal: bool,
) -> AppResult<TrainingPlan> {
    training::create(&*db.conn()?, &input, create_goal, now())
}

#[tauri::command(async)]
pub fn training_plans(db: State<'_, Db>, include_archived: bool) -> AppResult<Vec<PlanSummary>> {
    training::list(&*db.conn()?, now(), include_archived)
}

#[tauri::command(async)]
pub fn training_plan(db: State<'_, Db>, id: i64) -> AppResult<PlanDetail> {
    training::detail(&*db.conn()?, id, now())
}

#[tauri::command(async)]
pub fn rename_training_plan(db: State<'_, Db>, id: i64, name: String) -> AppResult<TrainingPlan> {
    training::rename(&*db.conn()?, id, &name)
}

#[tauri::command(async)]
pub fn adapt_training_plan(db: State<'_, Db>, id: i64) -> AppResult<PlanDetail> {
    training::adapt(&*db.conn()?, id, now())
}

#[tauri::command(async)]
pub fn archive_training_plan(
    db: State<'_, Db>,
    id: i64,
    archived: bool,
) -> AppResult<TrainingPlan> {
    training::set_archived(&*db.conn()?, id, archived)
}

#[tauri::command(async)]
pub fn delete_training_plan(db: State<'_, Db>, id: i64) -> AppResult<()> {
    training::delete(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn update_training_session(
    db: State<'_, Db>,
    id: i64,
    input: SessionInput,
) -> AppResult<TrainingSession> {
    training::update_session(&*db.conn()?, id, &input)
}

#[tauri::command(async)]
pub fn add_training_session(
    db: State<'_, Db>,
    plan_id: i64,
    input: SessionInput,
) -> AppResult<TrainingSession> {
    training::add_session(&*db.conn()?, plan_id, &input)
}

#[tauri::command(async)]
pub fn delete_training_session(db: State<'_, Db>, id: i64) -> AppResult<()> {
    training::delete_session(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn skip_training_session(
    db: State<'_, Db>,
    id: i64,
    skipped: bool,
) -> AppResult<TrainingSession> {
    training::skip_session(&*db.conn()?, id, skipped)
}

#[tauri::command(async)]
pub fn link_training_session(
    db: State<'_, Db>,
    id: i64,
    activity_id: Option<i64>,
    unlinked: bool,
) -> AppResult<TrainingSession> {
    training::link_session(&*db.conn()?, id, activity_id, unlinked)
}

#[tauri::command(async)]
pub fn training_link_candidates(db: State<'_, Db>, id: i64) -> AppResult<Vec<MatchedRun>> {
    training::link_candidates(&*db.conn()?, id)
}

/// Sessions from `from` to `to` ("YYYY-MM-DD", both included), for the dashboard.
#[tauri::command(async)]
pub fn training_sessions_between(
    db: State<'_, Db>,
    from: String,
    to: String,
) -> AppResult<Vec<PlanSession>> {
    training::sessions_between(&*db.conn()?, &from, &to, now())
}

#[tauri::command(async)]
pub fn activity_training_session(
    db: State<'_, Db>,
    activity_id: i64,
) -> AppResult<Option<PlanSession>> {
    training::activity_session(&*db.conn()?, activity_id, now())
}
