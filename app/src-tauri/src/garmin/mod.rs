//! Garmin Connect sync, the replacement for `importer/garmin/garmin.py`: downloads TCX files of
//! new activities and imports them like any other file.

pub mod client;
pub mod credentials;
pub mod oauth1;

use std::path::Path;

use chrono::{Local, NaiveDate};
use rusqlite::Connection;

use self::client::{ActivitySummary, Api, OAuth1Token, SignIn};
use crate::error::{AppError, AppResult};

/// Key in `sync.json`, as written by the Python app ("YYYY-MM-DD").
pub const LAST_SYNC_KEY: &str = "last_garmin_sync";

/// File name (without `.tcx`) of a downloaded activity, as the Python app named them, so
/// activities imported by either app are recognised as duplicates.
pub fn file_id(activity: &ActivitySummary) -> String {
    format!(
        "garmin_activity_{}_{}",
        activity.date(),
        activity.activity_id
    )
}

pub fn already_imported(conn: &Connection, file_id: &str) -> AppResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM activities WHERE file_id = ?1)",
        [file_id],
        |row| row.get(0),
    )?)
}

/// First day to ask Garmin for: the last synced day again (in case more activities followed
/// that day), or everything when this account was never synced.
pub fn start_date(last_sync: Option<&str>) -> String {
    last_sync
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map_or_else(|| "2000-01-01".to_string(), |d| d.to_string())
}

pub fn today() -> String {
    Local::now().date_naive().to_string()
}

/// Signs in and stores the long-lived token.
pub async fn sign_in(username: &str, password: &str) -> AppResult<SignIn> {
    let result = client::sign_in(username, password).await?;
    if let SignIn::Done(token) = &result {
        credentials::save_token(token)?;
    }
    Ok(result)
}

/// An API session from the stored token, or from a password the Python app stored.
pub async fn connect() -> AppResult<Api> {
    if let Some(token) = credentials::token()? {
        return Api::connect(&token).await;
    }
    let Some((username, password)) = credentials::legacy_login()? else {
        return Err(AppError::GarminSignInRequired);
    };
    match client::sign_in(&username, &password).await {
        Ok(SignIn::Done(token)) => {
            credentials::save_token(&token)?;
            Api::connect(&token).await
        }
        // A code has to be entered in the sign-in dialog.
        Ok(SignIn::NeedsCode(_)) | Err(_) => Err(AppError::GarminSignInRequired),
    }
}

/// Writes a downloaded TCX to `dir` under its file id, for the importer.
pub fn write_download(dir: &Path, file_id: &str, xml: &str) -> AppResult<std::path::PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{file_id}.tcx"));
    std::fs::write(&path, xml)?;
    Ok(path)
}

pub fn is_signed_in() -> AppResult<bool> {
    Ok(credentials::token()?.is_some() || credentials::legacy_login()?.is_some())
}

pub fn store_token(token: &OAuth1Token) -> AppResult<()> {
    credentials::save_token(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    #[test]
    fn names_files_like_python() {
        let activity = ActivitySummary {
            activity_id: 18_412_345_678,
            start_time_local: "2025-05-06 13:18:33".into(),
            activity_name: Some("Lunch run".into()),
        };
        assert_eq!(file_id(&activity), "garmin_activity_2025-05-06_18412345678");
    }

    #[test]
    fn detects_imported_files() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 1, "Running", 0, 5.0, "Run");
        conn.execute("UPDATE activities SET file_id = 'garmin_activity_x_1'", [])
            .unwrap();
        assert!(already_imported(&conn, "garmin_activity_x_1").unwrap());
        assert!(!already_imported(&conn, "garmin_activity_x_2").unwrap());
    }

    #[test]
    fn starts_at_last_sync_or_everything() {
        assert_eq!(start_date(Some("2025-05-06")), "2025-05-06");
        assert_eq!(start_date(Some("garbage")), "2000-01-01");
        assert_eq!(start_date(None), "2000-01-01");
    }
}
