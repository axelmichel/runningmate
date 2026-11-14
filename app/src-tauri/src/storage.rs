//! Data folder statistics for the system settings, after `utils/media_stats.py` and
//! `utils/folder_size.py`.

use std::{fs, path::Path};

use serde::Serialize;
use ts_rs::TS;

use crate::{error::AppResult, paths};

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StorageInfo {
    pub data_dir: String,
    /// Files in the media folder.
    #[ts(type = "number")]
    pub media_files: u64,
    /// Bytes.
    #[ts(type = "number")]
    pub media_size: u64,
    /// Bytes in the whole data folder.
    #[ts(type = "number")]
    pub total_size: u64,
}

/// (file count, bytes) below `dir`, following no symlinks; a missing folder counts as empty.
pub fn folder_stats(dir: &Path) -> (u64, u64) {
    let Ok(entries) = fs::read_dir(dir) else {
        return (0, 0);
    };
    entries
        .filter_map(Result::ok)
        .fold((0, 0), |(count, size), entry| match entry.file_type() {
            Ok(t) if t.is_dir() => {
                let (c, s) = folder_stats(&entry.path());
                (count + c, size + s)
            }
            Ok(t) if t.is_file() => (
                count + 1,
                size + entry.metadata().map(|m| m.len()).unwrap_or(0),
            ),
            _ => (count, size),
        })
}

pub fn info() -> AppResult<StorageInfo> {
    let data_dir = paths::data_dir()?;
    let (media_files, media_size) = folder_stats(&paths::media_dir()?);
    let (_, total_size) = folder_stats(&data_dir);
    Ok(StorageInfo {
        data_dir: data_dir.display().to_string(),
        media_files,
        media_size,
        total_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), [0u8; 10]).unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/b.mp4"), [0u8; 5]).unwrap();
        assert_eq!(folder_stats(dir.path()), (2, 15));
        assert_eq!(folder_stats(&dir.path().join("missing")), (0, 0));
    }
}
