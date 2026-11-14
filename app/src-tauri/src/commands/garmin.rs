//! Garmin Connect sign-in, sync (import) and upload (export). Both directions report through
//! the same `import://progress` events as file imports.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;
use ts_rs::TS;

use super::import::{ImportProgress, ImportSummary, PROGRESS_EVENT, import_one};
use crate::{
    db::{
        Db,
        connectors::{self, ConnectorLink, Direction, ExportCandidate},
    },
    error::{AppError, AppResult},
    garmin::{
        self,
        client::{PendingSignIn, SignIn, Upload},
    },
    importer::{self, ImportOutcome},
    paths, sync,
};

/// A sign-in waiting for its two-step verification code.
#[derive(Default)]
pub struct PendingGarminSignIn(Mutex<Option<PendingSignIn>>);

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GarminStatus {
    pub signed_in: bool,
    /// "YYYY-MM-DD" of the newest synced day.
    pub last_sync: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "status")]
#[ts(export)]
pub enum GarminSignInResult {
    SignedIn,
    NeedsCode,
}

#[tauri::command(async)]
pub fn garmin_status() -> AppResult<GarminStatus> {
    Ok(GarminStatus {
        signed_in: garmin::is_signed_in()?,
        last_sync: sync::value(garmin::LAST_SYNC_KEY)?,
    })
}

#[tauri::command]
pub async fn garmin_sign_in(
    pending: State<'_, PendingGarminSignIn>,
    username: String,
    password: String,
) -> AppResult<GarminSignInResult> {
    match garmin::sign_in(username.trim(), &password).await? {
        SignIn::Done(_) => Ok(GarminSignInResult::SignedIn),
        SignIn::NeedsCode(next) => {
            *pending.0.lock().await = Some(next);
            Ok(GarminSignInResult::NeedsCode)
        }
    }
}

#[tauri::command]
pub async fn garmin_submit_code(
    pending: State<'_, PendingGarminSignIn>,
    code: String,
) -> AppResult<()> {
    let next = pending
        .0
        .lock()
        .await
        .take()
        .ok_or_else(|| AppError::Invalid("no sign-in is waiting for a code".into()))?;
    let token = next.submit_code(&code).await?;
    garmin::store_token(&token)
}

#[tauri::command(async)]
pub fn garmin_sign_out() -> AppResult<()> {
    garmin::credentials::forget()
}

/// Downloads activities started since the last sync that aren't imported yet and imports them.
/// The sync date only advances past activities that could be downloaded, so failed downloads
/// are retried next time.
#[tauri::command]
pub async fn garmin_sync(app: AppHandle) -> AppResult<ImportSummary> {
    let api = garmin::connect().await?;
    let last_sync = sync::value(garmin::LAST_SYNC_KEY)?;
    let activities = api
        .activities(&garmin::start_date(last_sync.as_deref()), &garmin::today())
        .await?;
    let download_dir = std::env::temp_dir().join("runningmate-garmin");

    let mut summary = ImportSummary::default();
    let mut synced_until = last_sync.clone();
    let mut download_failed = false;
    for (index, activity) in activities.iter().enumerate() {
        let file_id = garmin::file_id(activity);
        let label = activity.activity_name.clone().map_or_else(
            || file_id.clone(),
            |name| format!("{name} ({})", activity.date()),
        );

        let existing = {
            let db = app.state::<Db>();
            let conn = db.conn()?;
            garmin::already_imported(&conn, &file_id)?
        };
        let outcome = if existing {
            let db = app.state::<Db>();
            let conn = db.conn()?;
            let activity_id = conn.query_row(
                "SELECT id FROM activities WHERE file_id = ?1",
                [&file_id],
                |row| row.get(0),
            )?;
            ImportOutcome::Duplicate { activity_id }
        } else {
            match api.download_tcx(activity.activity_id).await {
                Ok(xml) => {
                    let path = garmin::write_download(&download_dir, &file_id, &xml)?;
                    let app = app.clone();
                    let import_path = path.clone();
                    let outcome = tauri::async_runtime::spawn_blocking(move || {
                        import_one(&app, &import_path)
                    })
                    .await
                    .map_err(|e| AppError::Invalid(format!("import stopped: {e}")))??;
                    let _ = std::fs::remove_file(path);
                    outcome
                }
                Err(e) => {
                    download_failed = true;
                    ImportOutcome::Failed {
                        error: e.to_string(),
                    }
                }
            }
        };
        if let ImportOutcome::Imported { activity_id } | ImportOutcome::Duplicate { activity_id } =
            outcome
        {
            let db = app.state::<Db>();
            let conn = db.conn()?;
            let remote = activity.activity_id.to_string();
            connectors::link(
                &conn,
                activity_id,
                connectors::GARMIN,
                Some(&remote),
                Direction::Import,
            )?;
        }
        // Advance the sync date over everything handled so far, but not past a failed download.
        if !download_failed {
            synced_until = Some(activity.date().to_string());
        }
        summary.count(&outcome);
        let _ = app.emit(
            PROGRESS_EVENT,
            ImportProgress {
                done: index + 1,
                total: activities.len(),
                file: label,
                outcome,
            },
        );
    }
    if let Some(day) = synced_until.filter(|day| Some(day) != last_sync.as_ref()) {
        sync::set_value(garmin::LAST_SYNC_KEY, &day)?;
    }
    Ok(summary)
}

/// Activities since `since` (Unix timestamp) that could be uploaded to Garmin Connect.
#[tauri::command(async)]
pub fn garmin_export_candidates(db: State<'_, Db>, since: i64) -> AppResult<Vec<ExportCandidate>> {
    connectors::export_candidates(&*db.conn()?, connectors::GARMIN, since)
}

/// Uploads activities' archived TCX files to Garmin Connect, reporting each through
/// `import://progress` (imported = uploaded, duplicate = Garmin already had it).
#[tauri::command]
pub async fn garmin_upload(app: AppHandle, ids: Vec<i64>) -> AppResult<ImportSummary> {
    let api = garmin::connect().await?;
    let imports_dir = paths::imports_dir()?;
    let mut summary = ImportSummary::default();
    for (index, id) in ids.iter().enumerate() {
        let (title, file_id) = {
            let db = app.state::<Db>();
            let conn = db.conn()?;
            conn.query_row(
                "SELECT title, file_id FROM activities WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )?
        };
        let label = title.unwrap_or_else(|| format!("#{id}"));
        let result = async {
            let file_id =
                file_id.ok_or_else(|| AppError::NotFound(format!("TCX file of activity {id}")))?;
            let xml =
                importer::archive::read(&importer::archive::archive_path(&imports_dir, &file_id))?;
            api.upload_tcx(&format!("{file_id}.tcx"), xml).await
        }
        .await;
        let outcome = match result {
            Ok(upload) => {
                let remote = match upload {
                    Upload::Created(remote) | Upload::Duplicate(remote) => remote,
                };
                let db = app.state::<Db>();
                let conn = db.conn()?;
                connectors::link(
                    &conn,
                    *id,
                    connectors::GARMIN,
                    remote.map(|r| r.to_string()).as_deref(),
                    Direction::Export,
                )?;
                match upload {
                    Upload::Created(_) => ImportOutcome::Imported { activity_id: *id },
                    Upload::Duplicate(_) => ImportOutcome::Duplicate { activity_id: *id },
                }
            }
            Err(AppError::GarminSignInRequired) => return Err(AppError::GarminSignInRequired),
            Err(e) => ImportOutcome::Failed {
                error: e.to_string(),
            },
        };
        summary.count(&outcome);
        let _ = app.emit(
            PROGRESS_EVENT,
            ImportProgress {
                done: index + 1,
                total: ids.len(),
                file: label,
                outcome,
            },
        );
    }
    Ok(summary)
}

/// Services an activity is linked to (imported from or uploaded to).
#[tauri::command(async)]
pub fn activity_links(db: State<'_, Db>, id: i64) -> AppResult<Vec<ConnectorLink>> {
    connectors::links(&*db.conn()?, id)
}
