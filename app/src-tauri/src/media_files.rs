//! Adding photos and videos to an activity: copied into `~/RunningData/media`, or linked to the
//! original file where that is safe.

use std::{
    fs,
    path::{Path, PathBuf},
};

use rusqlite::Connection;

use crate::{
    db::media::{self, Media, MediaType},
    error::{AppError, AppResult},
};

/// Files that vanish or move on their own and must always be copied: temporary files (photos
/// dragged out of the Photos app, picker results) and files inside a Photos library package.
pub fn must_copy(path: &Path) -> bool {
    let temp = fs::canonicalize(std::env::temp_dir()).unwrap_or_else(|_| std::env::temp_dir());
    let file = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    file.starts_with(&temp)
        || file.starts_with("/private/var/folders")
        || file
            .components()
            .any(|c| c.as_os_str().to_string_lossy().ends_with(".photoslibrary"))
}

/// `dir/name`, or `dir/stem-N.ext` if that file already exists.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("media");
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    (1..)
        .map(|n| dir.join(format!("{stem}-{n}.{extension}")))
        .find(|p| !p.exists())
        .expect("an unused file name exists")
}

/// Adds a file to an activity. With `link`, the original is referenced instead of copied,
/// unless [`must_copy`] says it can't be.
pub fn add(
    conn: &Connection,
    media_dir: &Path,
    activity_id: i64,
    source: &Path,
    link: bool,
) -> AppResult<Media> {
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    let media_type = MediaType::from_extension(extension).ok_or_else(|| {
        AppError::Invalid(format!("unsupported media file: {}", source.display()))
    })?;
    if !source.is_file() {
        return Err(AppError::NotFound(source.display().to_string()));
    }
    let target = if link && !must_copy(source) {
        fs::canonicalize(source)?
    } else {
        let name = source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| AppError::Invalid(format!("invalid file name: {}", source.display())))?;
        fs::create_dir_all(media_dir)?;
        let target = unique_path(media_dir, name);
        fs::copy(source, &target)?;
        target
    };
    let file_path = target.to_string_lossy().into_owned();
    let id = media::insert(conn, activity_id, media_type, &file_path)?;
    Ok(Media {
        id,
        media_type,
        file_path,
        linked: false,
        missing: false,
    }
    .with_file_state(media_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn copies_or_links() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        // Outside the temp directory, so linking is allowed.
        let home = dirs::home_dir().unwrap();
        let originals = tempfile::tempdir_in(&home).unwrap();
        let media_dir = tempfile::tempdir().unwrap();
        let photo = originals.path().join("run.jpg");
        fs::write(&photo, b"jpg").unwrap();

        let copied = add(&conn, media_dir.path(), 1, &photo, false).unwrap();
        assert!(!copied.linked);
        assert!(
            Path::new(&copied.file_path).starts_with(fs::canonicalize(media_dir.path()).unwrap())
                || Path::new(&copied.file_path).starts_with(media_dir.path())
        );

        let linked = add(&conn, media_dir.path(), 1, &photo, true).unwrap();
        assert!(linked.linked);
        assert_eq!(
            linked.file_path,
            fs::canonicalize(&photo).unwrap().to_string_lossy()
        );

        assert!(matches!(
            add(
                &conn,
                media_dir.path(),
                1,
                &originals.path().join("notes.txt"),
                false
            ),
            Err(AppError::Invalid(_))
        ));
    }

    #[test]
    fn temporary_and_library_files_are_always_copied() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        assert!(must_copy(temp.path()));
        assert!(must_copy(Path::new(
            "/Users/me/Pictures/Photos Library.photoslibrary/originals/A/IMG_1.jpeg"
        )));
        assert!(!must_copy(Path::new("/Users/me/Pictures/run.jpg")));
    }

    #[test]
    fn unique_path_avoids_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(unique_path(dir.path(), "a.jpg"), dir.path().join("a.jpg"));
        fs::write(dir.path().join("a.jpg"), b"").unwrap();
        fs::write(dir.path().join("a-1.jpg"), b"").unwrap();
        assert_eq!(unique_path(dir.path(), "a.jpg"), dir.path().join("a-2.jpg"));
    }
}
