//! Garmin Connect sync, the replacement for `importer/garmin/garmin.py`: downloads TCX files of
//! new activities and imports them like any other file.

pub mod client;
pub mod credentials;
pub mod oauth1;

use std::path::Path;

use chrono::{Local, NaiveDate};

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

/// First day to ask Garmin for: the last synced day again (in case more activities followed
/// that day), or everything when this account was never synced.
pub fn start_date(last_sync: Option<&str>) -> String {
    last_sync
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map_or_else(|| "2000-01-01".to_string(), |d| d.to_string())
}

/// Key in `sync.json` for when a sync last ran to the end (RFC 3339); Rust app only.
pub const LAST_RUN_KEY: &str = "last_garmin_sync_run";

/// Where the next open-ended sync should start, after one that saw activities up to `newest`:
/// the oldest day with a failed download, so it is retried and nothing older is skipped, or
/// else the newest day. `None` when nothing changes.
pub fn resume_day(
    previous: Option<&str>,
    newest: Option<&str>,
    oldest_failed: Option<&str>,
) -> Option<String> {
    oldest_failed
        .or(newest)
        .filter(|day| Some(*day) != previous)
        .map(String::from)
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

    #[test]
    fn names_files_like_python() {
        let activity = ActivitySummary {
            activity_id: 18_412_345_678,
            start_time_local: "2025-05-06 13:18:33".into(),
            activity_name: Some("Lunch run".into()),
            distance: None,
        };
        assert_eq!(file_id(&activity), "garmin_activity_2025-05-06_18412345678");
    }

    #[test]
    fn resumes_at_the_oldest_failure_or_the_newest_day() {
        assert_eq!(
            resume_day(Some("2026-09-01"), Some("2026-09-27"), None).as_deref(),
            Some("2026-09-27")
        );
        assert_eq!(
            resume_day(None, Some("2026-09-27"), Some("2026-03-05")).as_deref(),
            Some("2026-03-05")
        );
        assert_eq!(
            resume_day(Some("2026-09-27"), Some("2026-09-27"), None),
            None
        );
        assert_eq!(resume_day(Some("2026-09-27"), None, None), None);
    }

    #[test]
    fn starts_at_last_sync_or_everything() {
        assert_eq!(start_date(Some("2025-05-06")), "2025-05-06");
        assert_eq!(start_date(Some("garbage")), "2000-01-01");
        assert_eq!(start_date(None), "2000-01-01");
    }
}
