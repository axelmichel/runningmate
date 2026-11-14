//! AI summaries of activities, written by a local Ollama model when enabled in the settings.

use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    ai::{self, AiQueue},
    analysis::comparison,
    db::{
        Db,
        ai_summaries::{self, AiSummary},
    },
    error::{AppError, AppResult},
    paths, settings,
};

/// Activities older than this aren't summarised automatically after import, so importing a
/// whole history doesn't queue hundreds of summaries.
const AUTO_DAYS: i64 = 14;

/// Models installed on the Ollama server at `url`.
#[tauri::command]
pub async fn ai_models(url: String) -> AppResult<Vec<String>> {
    ai::list_models(&url).await
}

#[tauri::command(async)]
pub fn get_ai_summary(db: State<'_, Db>, id: i64) -> AppResult<Option<AiSummary>> {
    ai_summaries::get(&*db.conn()?, id)
}

/// Writes (or rewrites) the summary of an activity and stores it.
#[tauri::command]
pub async fn generate_ai_summary(app: AppHandle, id: i64) -> AppResult<AiSummary> {
    generate(&app, id).await
}

async fn generate(app: &AppHandle, id: i64) -> AppResult<AiSummary> {
    let settings = settings::load()?;
    if !settings.ai_enabled || settings.ai_model.is_empty() {
        return Err(AppError::Invalid(
            "AI summaries are turned off or no model is chosen".into(),
        ));
    }
    let queue = app.state::<AiQueue>();
    let _turn = queue
        .0
        .acquire()
        .await
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    let facts = {
        // The facts are read with the connection held briefly, never across the request.
        let app = app.clone();
        let weeks = settings.ai_period_weeks;
        tauri::async_runtime::spawn_blocking(move || {
            comparison::comparison(
                &*app.state::<Db>().conn()?,
                id,
                &paths::imports_dir()?,
                weeks,
            )
        })
        .await
        .map_err(|e| AppError::Invalid(e.to_string()))??
    };
    let text = ai::summarize(
        &settings.ai_url,
        &settings.ai_model,
        &facts,
        &settings.language,
    )
    .await?;
    let summary = AiSummary {
        model: settings.ai_model,
        text,
        created: chrono::Utc::now().timestamp(),
    };
    ai_summaries::upsert(&*app.state::<Db>().conn()?, id, &summary)?;
    let _ = app.emit("activity://updated", id);
    Ok(summary)
}

/// Summarises a just imported activity in the background, if summaries are on, the activity is
/// recent and it has no summary yet. Failures are dropped: the user can still generate one.
pub(crate) fn summarize_later(app: &AppHandle, activity_id: i64) {
    let Ok(settings) = settings::load() else {
        return;
    };
    if !settings.ai_enabled || settings.ai_model.is_empty() {
        return;
    }
    let pending = app.state::<Db>().conn().is_ok_and(|conn| {
        let recent = crate::analysis::activity_basics(&conn, activity_id)
            .is_ok_and(|(_, date)| date >= chrono::Utc::now().timestamp() - AUTO_DAYS * 24 * 3600);
        recent && matches!(ai_summaries::get(&conn, activity_id), Ok(None))
    });
    if !pending {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = generate(&app, activity_id).await;
    });
}
