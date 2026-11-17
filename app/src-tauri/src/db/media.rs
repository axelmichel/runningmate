//! Photos and videos attached to an activity (`media` table).

use std::path::Path;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    activity_type::ViewMode,
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum MediaType {
    Image,
    Video,
}

impl MediaType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }

    /// Media type for a file extension, as accepted by the Python upload dialog.
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "heic" => Some(Self::Image),
            "mp4" | "mov" | "avi" => Some(Self::Video),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Media {
    #[ts(type = "number")]
    pub id: i64,
    pub media_type: MediaType,
    /// Absolute path: in `~/RunningData/media` for copies, anywhere for linked files.
    pub file_path: String,
    /// Refers to the original file instead of a copy in the media folder.
    pub linked: bool,
    /// The file no longer exists (e.g. a linked original that was moved).
    pub missing: bool,
}

impl Media {
    /// Fills `linked` and `missing` from the file system.
    pub fn with_file_state(mut self, media_dir: &Path) -> Self {
        let path = Path::new(&self.file_path);
        self.missing = !path.exists();
        let dir = std::fs::canonicalize(media_dir).unwrap_or_else(|_| media_dir.to_path_buf());
        let file = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        self.linked = !file.starts_with(dir);
        self
    }
}

fn from_row(row: &rusqlite::Row, media_dir: &Path) -> rusqlite::Result<Media> {
    let media_type: String = row.get("media_type")?;
    Ok(Media {
        id: row.get("id")?,
        media_type: if media_type == "video" {
            MediaType::Video
        } else {
            MediaType::Image
        },
        file_path: row.get("file_path")?,
        linked: false,
        missing: false,
    }
    .with_file_state(media_dir))
}

pub fn list(conn: &Connection, activity_id: i64, media_dir: &Path) -> AppResult<Vec<Media>> {
    let mut stmt = conn.prepare(
        "SELECT id, media_type, file_path FROM media WHERE activity_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map([activity_id], |row| from_row(row, media_dir))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A photo with the activity it belongs to.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActivityPhoto {
    #[ts(type = "number")]
    pub activity_id: i64,
    pub media: Media,
}

/// The `limit` latest photos of all activities, newest activity first; files that no longer
/// exist are skipped.
pub fn recent_photos(
    conn: &Connection,
    limit: usize,
    media_dir: &Path,
) -> AppResult<Vec<ActivityPhoto>> {
    let mut stmt = conn.prepare(
        "SELECT media.id, media.media_type, media.file_path, media.activity_id
         FROM media JOIN activities ON activities.id = media.activity_id
         WHERE media.media_type = 'image'
         ORDER BY activities.date DESC, media.id",
    )?;
    let mut photos = Vec::new();
    for row in stmt.query_map([], |row| {
        Ok(ActivityPhoto {
            activity_id: row.get("activity_id")?,
            media: from_row(row, media_dir)?,
        })
    })? {
        let photo = row?;
        if !photo.media.missing {
            photos.push(photo);
            if photos.len() == limit {
                break;
            }
        }
    }
    Ok(photos)
}

/// A photo or video with the activity it belongs to, for the gallery.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GalleryItem {
    #[ts(type = "number")]
    pub activity_id: i64,
    pub activity_title: Option<String>,
    /// Unix timestamp of the activity's start.
    #[ts(type = "number | null")]
    pub activity_date: Option<i64>,
    pub view_mode: ViewMode,
    pub media: Media,
}

/// Every photo and video, newest activity first and in the order added within one; files that
/// no longer exist are skipped.
pub fn gallery(conn: &Connection, media_dir: &Path) -> AppResult<Vec<GalleryItem>> {
    let mut stmt = conn.prepare(
        "SELECT media.id, media.media_type, media.file_path, media.activity_id,
            activities.title, activities.date, activities.activity_type
         FROM media JOIN activities ON activities.id = media.activity_id
         ORDER BY activities.date DESC, media.activity_id, media.id",
    )?;
    let items = stmt.query_map([], |row| {
        Ok(GalleryItem {
            activity_id: row.get("activity_id")?,
            activity_title: row.get("title")?,
            activity_date: row.get("date")?,
            view_mode: ViewMode::from_activity_type(
                &row.get::<_, Option<String>>("activity_type")?
                    .unwrap_or_default(),
            ),
            media: from_row(row, media_dir)?,
        })
    })?;
    let mut out = Vec::new();
    for item in items {
        let item = item?;
        if !item.media.missing {
            out.push(item);
        }
    }
    Ok(out)
}

pub fn insert(
    conn: &Connection,
    activity_id: i64,
    media_type: MediaType,
    file_path: &str,
) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO media (activity_id, media_type, file_path) VALUES (?1, ?2, ?3)",
        params![activity_id, media_type.as_str(), file_path],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Deletes the row and returns the file path, so the caller can remove the file.
pub fn delete(conn: &Connection, id: i64) -> AppResult<String> {
    let path: String = conn
        .query_row("SELECT file_path FROM media WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("media {id}")),
            e => e.into(),
        })?;
    conn.execute("DELETE FROM media WHERE id = ?1", [id])?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn inserts_lists_and_deletes() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let media_dir = dir.path().join("media");
        std::fs::create_dir_all(&media_dir).unwrap();
        let copy = media_dir.join("clip.mp4");
        std::fs::write(&copy, b"").unwrap();
        let id = insert(&conn, 5, MediaType::Video, copy.to_str().unwrap()).unwrap();
        insert(&conn, 5, MediaType::Image, "/elsewhere/a.jpg").unwrap();
        let media = list(&conn, 5, &media_dir).unwrap();
        assert_eq!(media.len(), 2);
        assert_eq!(media[0].media_type, MediaType::Video);
        assert!(!media[0].linked && !media[0].missing);
        assert!(media[1].linked && media[1].missing);

        assert_eq!(delete(&conn, id).unwrap(), copy.to_str().unwrap());
        assert_eq!(list(&conn, 5, &media_dir).unwrap().len(), 1);
        assert!(matches!(delete(&conn, id), Err(AppError::NotFound(_))));
    }

    #[test]
    fn recent_photos_come_newest_activity_first() {
        use crate::db::activities::tests::insert_activity;

        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = |name: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, b"").unwrap();
            path.to_str().unwrap().to_string()
        };
        insert_activity(&conn, 1, "Running", 1_000, 5.0, "Older");
        insert_activity(&conn, 2, "Running", 2_000, 5.0, "Newer");
        insert(&conn, 1, MediaType::Image, &file("old.jpg")).unwrap();
        insert(&conn, 2, MediaType::Image, &file("new.jpg")).unwrap();
        insert(&conn, 2, MediaType::Video, &file("clip.mp4")).unwrap();
        insert(&conn, 2, MediaType::Image, "/gone/missing.jpg").unwrap();

        let photos = recent_photos(&conn, 10, dir.path()).unwrap();
        let ids: Vec<_> = photos.iter().map(|p| p.activity_id).collect();
        assert_eq!(ids, [2, 1]);
        assert!(photos[0].media.file_path.ends_with("new.jpg"));
        assert_eq!(recent_photos(&conn, 1, dir.path()).unwrap().len(), 1);
    }

    #[test]
    fn the_gallery_has_photos_and_videos_with_their_activity() {
        use crate::db::activities::tests::insert_activity;

        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = |name: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, b"").unwrap();
            path.to_str().unwrap().to_string()
        };
        insert_activity(&conn, 1, "Biking", 1_000, 40.0, "Ride");
        insert_activity(&conn, 2, "Running", 2_000, 5.0, "Run");
        insert(&conn, 1, MediaType::Image, &file("ride.jpg")).unwrap();
        insert(&conn, 2, MediaType::Image, &file("run.jpg")).unwrap();
        insert(&conn, 2, MediaType::Video, &file("run.mp4")).unwrap();
        insert(&conn, 2, MediaType::Image, "/gone/missing.jpg").unwrap();

        let items = gallery(&conn, dir.path()).unwrap();
        let ids: Vec<_> = items.iter().map(|i| i.activity_id).collect();
        assert_eq!(ids, [2, 2, 1]);
        assert_eq!(items[0].activity_title.as_deref(), Some("Run"));
        assert_eq!(items[1].media.media_type, MediaType::Video);
        assert_eq!(items[2].view_mode, ViewMode::Cycling);
    }

    #[test]
    fn media_type_from_extension() {
        assert_eq!(MediaType::from_extension("JPG"), Some(MediaType::Image));
        assert_eq!(MediaType::from_extension("mp4"), Some(MediaType::Video));
        assert_eq!(MediaType::from_extension("txt"), None);
    }
}
