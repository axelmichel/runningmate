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
    /// Archived activity files (`{file_id}.tcx.tar.gz`, with a GPX file they were converted from).
    #[ts(type = "number")]
    pub activity_files: u64,
    /// Bytes of the archives, the originals kept by fixes and files waiting for a route.
    #[ts(type = "number")]
    pub activity_size: u64,
    /// Originals kept by the fix editor (`{file_id}.orig.tcx.tar.gz`), included in the size.
    #[ts(type = "number")]
    pub kept_originals: u64,
    /// Bytes.
    #[ts(type = "number")]
    pub kept_originals_size: u64,
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

/// (archives, kept originals, kept originals' bytes) directly in the imports folder.
fn archive_stats(dir: &Path) -> (u64, u64, u64) {
    let Ok(entries) = fs::read_dir(dir) else {
        return (0, 0, 0);
    };
    entries
        .filter_map(Result::ok)
        .fold((0, 0, 0), |(archives, originals, bytes), entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".orig.tcx.tar.gz") {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                (archives, originals + 1, bytes + size)
            } else if name.ends_with(".tcx.tar.gz") {
                (archives + 1, originals, bytes)
            } else {
                (archives, originals, bytes)
            }
        })
}

pub fn info() -> AppResult<StorageInfo> {
    let data_dir = paths::data_dir()?;
    let (media_files, media_size) = folder_stats(&paths::media_dir()?);
    let imports = paths::imports_dir()?;
    let (_, activity_size) = folder_stats(&imports);
    let (activity_files, kept_originals, kept_originals_size) = archive_stats(&imports);
    let (_, total_size) = folder_stats(&data_dir);
    Ok(StorageInfo {
        data_dir: data_dir.display().to_string(),
        media_files,
        media_size,
        activity_files,
        activity_size,
        kept_originals,
        kept_originals_size,
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

    #[test]
    fn tells_archives_from_kept_originals() {
        let dir = tempfile::tempdir().unwrap();
        for (name, size) in [
            ("run.tcx.tar.gz", 10),
            ("ride.tcx.tar.gz", 20),
            ("run.orig.tcx.tar.gz", 7),
            ("notes.txt", 3),
        ] {
            fs::write(dir.path().join(name), vec![0u8; size]).unwrap();
        }
        assert_eq!(archive_stats(dir.path()), (2, 1, 7));
        assert_eq!(archive_stats(&dir.path().join("missing")), (0, 0, 0));
    }
}
