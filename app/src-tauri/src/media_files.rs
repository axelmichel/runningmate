//! Adding photos and videos to an activity: copied into `~/RunningData/media`, or linked to the
//! original file where that is safe. Copied photos are scaled down as the photo size setting
//! says ([`photo_shrink`]); copies already stored can be shrunk later the same way.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

use serde::Serialize;
use ts_rs::TS;

use crate::{
    db::{
        Db,
        media::{self, Media, MediaType},
    },
    error::{AppError, AppResult},
    photo_shrink::{self, Shrunk},
    settings::PhotoSize,
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

/// `name` with the extension of a shrunk version: unchanged if it already fits (`.JPEG` stays
/// as it is for a JPEG), else the stem with the new extension. The stem stays, so Photos library
/// items are still recognised as added.
fn name_for(name: &str, extension: &str) -> String {
    let path = Path::new(name);
    let current = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let same = current == extension || (extension == "jpg" && current == "jpeg");
    if same {
        return name.to_string();
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("photo");
    format!("{stem}.{extension}")
}

/// Moves a shrunk version into place at `target`; the temp file is removed if that fails.
fn put(shrunk: &Shrunk, target: &Path) -> AppResult<()> {
    fs::rename(&shrunk.temp, target).inspect_err(|_| {
        let _ = fs::remove_file(&shrunk.temp);
    })?;
    Ok(())
}

/// Copies `source` into the media folder: photos as a smaller version if the size setting
/// shrinks them, the original if not (or if shrinking fails).
fn copy_into(
    media_dir: &Path,
    source: &Path,
    name: &str,
    media_type: MediaType,
    photo_size: PhotoSize,
) -> AppResult<PathBuf> {
    if media_type == MediaType::Image {
        match photo_shrink::shrink(source, photo_size, media_dir) {
            Ok(Some(shrunk)) => {
                let target = unique_path(media_dir, &name_for(name, shrunk.extension));
                put(&shrunk, &target)?;
                return Ok(target);
            }
            Ok(None) => {}
            Err(e) => eprintln!(
                "shrinking {} failed, keeping it as is: {e}",
                source.display()
            ),
        }
    }
    let target = unique_path(media_dir, name);
    fs::copy(source, &target)?;
    Ok(target)
}

/// Adds a file to an activity. With `link`, the original is referenced instead of copied,
/// unless [`must_copy`] says it can't be. Copied photos are shrunk to `photo_size`; the
/// database is only locked to insert the row, not while a photo is scaled.
pub fn add(
    db: &Db,
    media_dir: &Path,
    activity_id: i64,
    source: &Path,
    link: bool,
    photo_size: PhotoSize,
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
        copy_into(media_dir, source, name, media_type, photo_size)?
    };
    let file_path = target.to_string_lossy().into_owned();
    let id = media::insert(&*db.conn()?, activity_id, media_type, &file_path)?;
    Ok(Media {
        id,
        media_type,
        file_path,
        linked: false,
        missing: false,
    }
    .with_file_state(media_dir))
}

/// Copied photos larger than the size setting allows, and how much space they take.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShrinkPreview {
    pub photos: usize,
    /// Bytes.
    #[ts(type = "number")]
    pub bytes: u64,
}

/// Reported after each photo of [`shrink_stored`].
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShrinkProgress {
    pub done: usize,
    pub total: usize,
    /// Bytes saved so far.
    #[ts(type = "number")]
    pub saved: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShrinkSummary {
    /// Photos replaced by a smaller version.
    pub shrunk: usize,
    /// Photos left as they were, because a smaller version wouldn't have been much smaller.
    pub kept: usize,
    /// Photos that couldn't be read or replaced; they are unchanged.
    pub failed: usize,
    /// Bytes.
    #[ts(type = "number")]
    pub saved: u64,
}

/// Photos copied into the media folder whose longest side is above what `photo_size` allows.
/// Linked originals are never included.
pub fn photos_to_shrink(db: &Db, media_dir: &Path, photo_size: PhotoSize) -> AppResult<Vec<Media>> {
    let Some((max_side, _)) = photo_size.limits() else {
        return Ok(Vec::new());
    };
    let photos: Vec<Media> = media::copied_photos(&*db.conn()?, media_dir)?
        .into_iter()
        .filter(|photo| photo_shrink::supported(Path::new(&photo.file_path)))
        .collect();
    let paths: Vec<PathBuf> = photos.iter().map(|p| PathBuf::from(&p.file_path)).collect();
    let sides = photo_shrink::longest_sides(&paths);
    Ok(photos
        .into_iter()
        .zip(sides)
        .filter(|(_, side)| side.is_some_and(|side| side > max_side))
        .map(|(photo, _)| photo)
        .collect())
}

pub fn preview(photos: &[Media]) -> ShrinkPreview {
    ShrinkPreview {
        photos: photos.len(),
        bytes: photos
            .iter()
            .filter_map(|p| fs::metadata(&p.file_path).ok())
            .map(|m| m.len())
            .sum(),
    }
}

/// Puts a shrunk version in place of the stored copy at `path` and returns the bytes saved.
/// With the same extension it is renamed over the copy in one step. Otherwise it gets a free
/// name next to it, the rows are pointed at that, and only then is the old copy removed, so the
/// database always refers to a complete file.
fn replace(db: &Db, path: &Path, shrunk: Shrunk) -> AppResult<u64> {
    let before = fs::metadata(path)?.len();
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let new_name = name_for(name, shrunk.extension);
    if new_name == name {
        put(&shrunk, path)?;
    } else {
        let dir = path.parent().unwrap_or(Path::new("."));
        let target = unique_path(dir, &new_name);
        put(&shrunk, &target)?;
        let moved = db.conn().and_then(|conn| {
            media::move_file(&conn, &path.to_string_lossy(), &target.to_string_lossy())
        });
        if let Err(e) = moved {
            let _ = fs::remove_file(&target);
            return Err(e);
        }
        if let Err(e) = fs::remove_file(path) {
            eprintln!("removing the replaced photo {} failed: {e}", path.display());
        }
    }
    Ok(before.saturating_sub(shrunk.bytes))
}

/// Replaces the stored copies by smaller versions, scaling up to three at once (a decoded
/// 12-megapixel photo takes about 36 MB); `progress` is called after each photo. A photo that
/// fails is left as it was.
pub fn shrink_stored(
    db: &Db,
    media_dir: &Path,
    photos: &[Media],
    photo_size: PhotoSize,
    mut progress: impl FnMut(ShrinkProgress),
) -> ShrinkSummary {
    let mut summary = ShrinkSummary::default();
    let workers = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .clamp(1, 3)
        .min(photos.len().max(1));
    let next = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let sender = sender.clone();
            let next = &next;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(photo) = photos.get(index) else {
                        break;
                    };
                    let shrunk =
                        photo_shrink::shrink(Path::new(&photo.file_path), photo_size, media_dir);
                    if sender.send((photo, shrunk)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        for (done, (photo, shrunk)) in receiver.into_iter().enumerate() {
            let path = Path::new(&photo.file_path);
            match shrunk.and_then(|s| s.map(|s| replace(db, path, s)).transpose()) {
                Ok(Some(saved)) => {
                    summary.shrunk += 1;
                    summary.saved += saved;
                }
                Ok(None) => summary.kept += 1,
                Err(e) => {
                    eprintln!("shrinking {} failed: {e}", path.display());
                    summary.failed += 1;
                }
            }
            progress(ShrinkProgress {
                done: done + 1,
                total: photos.len(),
                saved: summary.saved,
            });
        }
    });
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn copies_or_links() {
        let db = Db::open_in_memory().unwrap();
        // Outside the temp directory, so linking is allowed.
        let home = dirs::home_dir().unwrap();
        let originals = tempfile::tempdir_in(&home).unwrap();
        let media_dir = tempfile::tempdir().unwrap();
        let photo = originals.path().join("run.jpg");
        fs::write(&photo, b"jpg").unwrap();

        // Not a real photo: shrinking fails and the original is copied.
        let copied = add(&db, media_dir.path(), 1, &photo, false, PhotoSize::Small).unwrap();
        assert!(!copied.linked);
        assert!(
            Path::new(&copied.file_path).starts_with(fs::canonicalize(media_dir.path()).unwrap())
                || Path::new(&copied.file_path).starts_with(media_dir.path())
        );
        assert_eq!(fs::read(&copied.file_path).unwrap(), b"jpg");

        let linked = add(&db, media_dir.path(), 1, &photo, true, PhotoSize::Small).unwrap();
        assert!(linked.linked);
        assert_eq!(
            linked.file_path,
            fs::canonicalize(&photo).unwrap().to_string_lossy()
        );

        assert!(matches!(
            add(
                &db,
                media_dir.path(),
                1,
                &originals.path().join("notes.txt"),
                false,
                PhotoSize::Small,
            ),
            Err(AppError::Invalid(_))
        ));
    }

    /// A large PNG photo outside the temp directory, so it may be linked.
    fn large_png(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        image::DynamicImage::ImageRgb8(photo_shrink::tests::photo(3000, 2000))
            .save(&path)
            .unwrap();
        path
    }

    fn longest_side(path: &str) -> u32 {
        photo_shrink::longest_sides(&[PathBuf::from(path)])[0].unwrap()
    }

    #[test]
    fn copied_photos_are_shrunk_and_linked_ones_left_alone() {
        let db = Db::open_in_memory().unwrap();
        let home = dirs::home_dir().unwrap();
        let originals = tempfile::tempdir_in(&home).unwrap();
        let media_dir = tempfile::tempdir().unwrap();
        let source = large_png(originals.path(), "summit.png");
        let before = fs::read(&source).unwrap();

        let copied = add(&db, media_dir.path(), 1, &source, false, PhotoSize::Small).unwrap();
        assert!(
            copied.file_path.ends_with("summit.jpg"),
            "{}",
            copied.file_path
        );
        assert_eq!(longest_side(&copied.file_path), 1920);

        let kept = add(
            &db,
            media_dir.path(),
            1,
            &source,
            false,
            PhotoSize::Original,
        )
        .unwrap();
        assert!(kept.file_path.ends_with("summit.png"));
        assert_eq!(fs::read(&kept.file_path).unwrap(), before);

        let linked = add(&db, media_dir.path(), 1, &source, true, PhotoSize::Small).unwrap();
        assert!(linked.linked);
        assert_eq!(fs::read(&source).unwrap(), before);
        // No temp files left behind.
        assert_eq!(fs::read_dir(media_dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn stored_copies_are_shrunk_in_place_and_renamed_in_the_database() {
        let db = Db::open_in_memory().unwrap();
        let home = dirs::home_dir().unwrap();
        let originals = tempfile::tempdir_in(&home).unwrap();
        let media_dir = tempfile::tempdir().unwrap();
        let source = large_png(originals.path(), "finish.png");
        let original_bytes = fs::read(&source).unwrap();
        // Stored before the setting existed: full size copies, plus a link and a video.
        let png = add(
            &db,
            media_dir.path(),
            1,
            &source,
            false,
            PhotoSize::Original,
        )
        .unwrap();
        let jpeg = media_dir.path().join("start.jpg");
        photo_shrink::tests::write_jpeg(&jpeg, &photo_shrink::tests::photo(4000, 3000), 95);
        let jpeg_id = media::insert(
            &db.conn().unwrap(),
            1,
            MediaType::Image,
            jpeg.to_str().unwrap(),
        )
        .unwrap();
        let small = media_dir.path().join("small.jpg");
        photo_shrink::tests::write_jpeg(&small, &photo_shrink::tests::photo(800, 600), 95);
        media::insert(
            &db.conn().unwrap(),
            1,
            MediaType::Image,
            small.to_str().unwrap(),
        )
        .unwrap();
        add(&db, media_dir.path(), 1, &source, true, PhotoSize::Original).unwrap();
        let video = media_dir.path().join("clip.mp4");
        fs::write(&video, b"mp4").unwrap();
        media::insert(
            &db.conn().unwrap(),
            1,
            MediaType::Video,
            video.to_str().unwrap(),
        )
        .unwrap();

        let photos = photos_to_shrink(&db, media_dir.path(), PhotoSize::Recommended).unwrap();
        let mut names: Vec<_> = photos.iter().map(|p| p.file_path.clone()).collect();
        names.sort();
        assert_eq!(
            names,
            [png.file_path.clone(), jpeg.to_string_lossy().into_owned()]
        );
        assert!(
            photos_to_shrink(&db, media_dir.path(), PhotoSize::Original)
                .unwrap()
                .is_empty()
        );
        let preview = preview(&photos);
        assert_eq!(preview.photos, 2);

        let mut reports = Vec::new();
        let summary = shrink_stored(
            &db,
            media_dir.path(),
            &photos,
            PhotoSize::Recommended,
            |p| reports.push(p),
        );
        assert_eq!(summary.shrunk, 2);
        assert_eq!(summary.failed, 0);
        assert!(summary.saved > 0);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[1].done, 2);
        assert_eq!(reports[1].saved, summary.saved);

        let rows = media::list(&db.conn().unwrap(), 1, media_dir.path()).unwrap();
        // The PNG became a JPEG: new name in the database, old file gone.
        let renamed = &rows.iter().find(|m| m.id == png.id).unwrap().file_path;
        assert!(renamed.ends_with("finish.jpg"), "{renamed}");
        assert_eq!(longest_side(renamed), 2560);
        assert!(!Path::new(&png.file_path).exists());
        // The JPEG kept its name.
        let same = &rows.iter().find(|m| m.id == jpeg_id).unwrap().file_path;
        assert_eq!(same, &jpeg.to_string_lossy());
        assert_eq!(longest_side(same), 2560);
        // The linked original, the small photo and the video are untouched.
        assert_eq!(fs::read(&source).unwrap(), original_bytes);
        assert_eq!(longest_side(small.to_str().unwrap()), 800);
        assert_eq!(fs::read(&video).unwrap(), b"mp4");
        assert!(rows.iter().all(|m| !m.missing));
        assert!(
            photos_to_shrink(&db, media_dir.path(), PhotoSize::Recommended)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn names_follow_the_new_format() {
        assert_eq!(name_for("a.heic", "jpg"), "a.jpg");
        assert_eq!(name_for("a.PNG", "jpg"), "a.jpg");
        assert_eq!(name_for("a.JPEG", "jpg"), "a.JPEG");
        assert_eq!(name_for("a.png", "png"), "a.png");
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
