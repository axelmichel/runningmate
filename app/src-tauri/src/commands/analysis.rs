use tauri::State;

use crate::{
    analysis::{
        self, HeartRateTrend, SegmentRow, fitness::FitnessDay, health::HealthReport,
        series::TrackPoint, training_effect::TrainingEffect, zones::ZoneTime,
    },
    db::Db,
    error::AppResult,
    paths,
};

/// Points for the statistics charts and the map; `None` without an archived TCX file.
#[tauri::command(async)]
pub fn activity_series(db: State<'_, Db>, id: i64) -> AppResult<Option<Vec<TrackPoint>>> {
    let track = analysis::load_track(&*db.conn()?, id, &paths::imports_dir()?)?;
    Ok(track.as_ref().map(analysis::series::series))
}

#[tauri::command(async)]
pub fn activity_segments(db: State<'_, Db>, id: i64) -> AppResult<Vec<SegmentRow>> {
    analysis::segments(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn activity_heart_rate_zones(db: State<'_, Db>, id: i64) -> AppResult<Option<Vec<ZoneTime>>> {
    analysis::heart_rate_zones(&*db.conn()?, id, &paths::imports_dir()?)
}

#[tauri::command(async)]
pub fn activity_training_effect(db: State<'_, Db>, id: i64) -> AppResult<TrainingEffect> {
    analysis::training_effect(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn fitness_trend(db: State<'_, Db>, id: i64) -> AppResult<Vec<FitnessDay>> {
    analysis::fitness_trend(&*db.conn()?, id)
}

#[tauri::command(async)]
pub fn heart_rate_trend(db: State<'_, Db>, id: i64) -> AppResult<HeartRateTrend> {
    analysis::heart_rate_trend(&*db.conn()?, id)
}

/// Fitness and heart rate between two Unix times, for the "Health data" page.
#[tauri::command(async)]
pub fn health_report(db: State<'_, Db>, from: i64, to: i64) -> AppResult<HealthReport> {
    analysis::health::report(&*db.conn()?, from, to)
}
