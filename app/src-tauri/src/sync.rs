//! Folder sync (e.g. an iCloud export folder) with its state in
//! `~/RunningData/appdata/sync.json`, shared with the Python app (`ui/window_icloud_sync.py`).

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use chrono::{DateTime, Local, NaiveDateTime};
use serde::Serialize;
use serde_json::{Map, Value};
use ts_rs::TS;

use crate::{error::AppResult, paths};

const FOLDER_KEY: &str = "icloud_folder";
const LAST_SYNC_KEY: &str = "last_icloud_sync";
/// Local time, as Python wrote it.
const LAST_SYNC_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FolderSyncStatus {
    pub folder: Option<String>,
    /// Unix timestamp of the last sync.
    #[ts(type = "number | null")]
    pub last_sync: Option<i64>,
    /// `.tcx` files changed since the last sync.
    pub new_files: Vec<String>,
}

fn sync_file() -> AppResult<PathBuf> {
    Ok(paths::appdata_dir()?.join("sync.json"))
}

fn read(path: &Path) -> AppResult<Map<String, Value>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e.into()),
    }
}

fn write_value(path: &Path, key: &str, value: Value) -> AppResult<()> {
    let mut map = read(path)?;
    map.insert(key.into(), value);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_string(&map)?)?;
    Ok(())
}

fn last_sync_from(map: &Map<String, Value>) -> Option<DateTime<Local>> {
    let text = map.get(LAST_SYNC_KEY)?.as_str()?;
    NaiveDateTime::parse_from_str(text, LAST_SYNC_FORMAT)
        .ok()?
        .and_local_timezone(Local)
        .earliest()
}

/// `.tcx` files directly in `folder` modified after `since` (all of them without a date).
pub fn changed_tcx_files(folder: &Path, since: Option<SystemTime>) -> AppResult<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("tcx"))
        })
        .filter(|path| match since {
            None => true,
            Some(since) => fs::metadata(path)
                .and_then(|m| m.modified())
                .is_ok_and(|modified| modified > since),
        })
        .collect();
    files.sort();
    Ok(files)
}

fn status_from(path: &Path) -> AppResult<FolderSyncStatus> {
    let map = read(path)?;
    let folder = map
        .get(FOLDER_KEY)
        .and_then(Value::as_str)
        .map(String::from);
    let last_sync = last_sync_from(&map);
    let new_files = match &folder {
        Some(folder) if Path::new(folder).is_dir() => {
            changed_tcx_files(Path::new(folder), last_sync.map(SystemTime::from))?
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect()
        }
        _ => Vec::new(),
    };
    Ok(FolderSyncStatus {
        folder,
        last_sync: last_sync.map(|t| t.timestamp()),
        new_files,
    })
}

pub fn status() -> AppResult<FolderSyncStatus> {
    status_from(&sync_file()?)
}

pub fn set_folder(folder: &str) -> AppResult<FolderSyncStatus> {
    let path = sync_file()?;
    write_value(&path, FOLDER_KEY, Value::String(folder.into()))?;
    status_from(&path)
}

/// A string value from `sync.json` (e.g. `last_garmin_sync`).
pub fn value(key: &str) -> AppResult<Option<String>> {
    Ok(read(&sync_file()?)?
        .get(key)
        .and_then(Value::as_str)
        .map(String::from))
}

pub fn set_value(key: &str, value: &str) -> AppResult<()> {
    write_value(&sync_file()?, key, Value::String(value.into()))
}

/// Records `time` as the last sync.
pub fn mark_synced(time: DateTime<Local>) -> AppResult<()> {
    write_value(
        &sync_file()?,
        LAST_SYNC_KEY,
        Value::String(time.format(LAST_SYNC_FORMAT).to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_changed_tcx_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.tcx", "b.TCX", "notes.txt"] {
            fs::write(dir.path().join(name), "").unwrap();
        }
        fs::create_dir(dir.path().join("sub.tcx")).unwrap();
        let all = changed_tcx_files(dir.path(), None).unwrap();
        assert_eq!(all.len(), 2);
        let future = SystemTime::now() + std::time::Duration::from_secs(60);
        assert!(
            changed_tcx_files(dir.path(), Some(future))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn reads_python_sync_file_and_keeps_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sync.json");
        fs::write(
            &path,
            format!(
                r#"{{"icloud_folder": "{}", "last_icloud_sync": "2025-05-06 22:54:24", "last_garmin_sync": "x"}}"#,
                dir.path().display()
            ),
        )
        .unwrap();
        let status = status_from(&path).unwrap();
        assert!(status.last_sync.is_some());
        assert_eq!(status.folder.as_deref(), dir.path().to_str());

        write_value(&path, FOLDER_KEY, Value::String("/elsewhere".into())).unwrap();
        let map = read(&path).unwrap();
        assert_eq!(map["last_garmin_sync"], "x");
        assert_eq!(map[FOLDER_KEY], "/elsewhere");
    }
}
