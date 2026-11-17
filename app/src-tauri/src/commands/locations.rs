//! Place names of activity starts, looked up one after another in the background: after
//! imports and recomputes, and for older activities from app start (newest first). Progress is
//! kept in the database, so a stopped lookup carries on where it left off.

use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};

use crate::{
    db::{Db, locations},
    nominatim::{self, LookupError},
    settings,
};

/// Sent with the activity id when its place name was stored.
pub const LOCATED_EVENT: &str = "activity://located";
/// How long lookups rest after the service couldn't be reached.
const PAUSE: Duration = Duration::from_secs(30 * 60);

static RUNNING: AtomicBool = AtomicBool::new(false);
/// Set by every request to look up; the running worker checks it before stopping.
static REQUESTED: AtomicBool = AtomicBool::new(false);
static PAUSED_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

fn paused() -> bool {
    PAUSED_UNTIL
        .lock()
        .map(|until| until.is_some_and(|until| Instant::now() < until))
        .unwrap_or(false)
}

/// Looks up the place names still missing, unless the lookup is switched off. Only one worker
/// runs; calls while it runs make it check for new work before it stops.
pub(crate) fn locate_later(app: &AppHandle) {
    REQUESTED.store(true, Ordering::SeqCst);
    if RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            REQUESTED.store(false, Ordering::SeqCst);
            locate_all(&app).await;
            RUNNING.store(false, Ordering::SeqCst);
            if !REQUESTED.load(Ordering::SeqCst) || RUNNING.swap(true, Ordering::SeqCst) {
                break;
            }
        }
    });
}

/// Clears the place name of a recomputed activity whose start moved, and looks it up again.
pub(crate) fn relocate_later(app: &AppHandle, activity_id: i64) {
    if let Ok(conn) = app.state::<Db>().conn() {
        let _ = locations::forget_if_moved(&conn, activity_id);
    }
    locate_later(app);
}

/// Works through the activities without a place name until none is left, the lookup is
/// switched off or the service can't be reached.
async fn locate_all(app: &AppHandle) {
    loop {
        // Read each time, so switching the lookup off stops it at once.
        let Ok(settings) = settings::load() else {
            return;
        };
        if !settings.location_lookup || paused() {
            return;
        }
        let next = app.state::<Db>().conn().ok().and_then(|conn| {
            let candidate = locations::next_candidate(&conn).ok()??;
            let nearby = locations::nearby_name(&conn, &candidate).ok()?;
            Some((candidate, nearby))
        });
        let Some((candidate, nearby)) = next else {
            return;
        };
        let name = match nearby {
            Some(name) => Some(name),
            None => {
                match nominatim::lookup(candidate.latitude, candidate.longitude, &settings.language)
                    .await
                {
                    Ok(name) => name,
                    // Marked as looked up, so it isn't asked for again.
                    Err(LookupError::Failed(e)) => {
                        eprintln!("place name of activity {}: {e}", candidate.activity_id);
                        None
                    }
                    Err(LookupError::Unavailable(e)) => {
                        eprintln!("place names paused: {e}");
                        if let Ok(mut until) = PAUSED_UNTIL.lock() {
                            *until = Some(Instant::now() + PAUSE);
                        }
                        return;
                    }
                }
            }
        };
        let stored = app
            .state::<Db>()
            .conn()
            .and_then(|conn| locations::store(&conn, &candidate, name.as_deref()));
        if stored.is_err() {
            return;
        }
        if name.is_some() {
            let _ = app.emit(LOCATED_EVENT, candidate.activity_id);
        }
    }
}
