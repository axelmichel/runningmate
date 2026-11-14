//! Photos and videos attached to an activity (`media` table).

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult};

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
    /// Absolute path of the file in `~/RunningData/media`.
    pub file_path: String,
}

pub fn list(conn: &Connection, activity_id: i64) -> AppResult<Vec<Media>> {
    let mut stmt = conn.prepare(
        "SELECT id, media_type, file_path FROM media WHERE activity_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map([activity_id], |row| {
        let media_type: String = row.get("media_type")?;
        Ok(Media {
            id: row.get("id")?,
            media_type: if media_type == "video" {
                MediaType::Video
            } else {
                MediaType::Image
            },
            file_path: row.get("file_path")?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
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
        let id = insert(&conn, 5, MediaType::Video, "/m/clip.mp4").unwrap();
        insert(&conn, 5, MediaType::Image, "/m/a.jpg").unwrap();
        let media = list(&conn, 5).unwrap();
        assert_eq!(media.len(), 2);
        assert_eq!(media[0].media_type, MediaType::Video);

        assert_eq!(delete(&conn, id).unwrap(), "/m/clip.mp4");
        assert_eq!(list(&conn, 5).unwrap().len(), 1);
        assert!(matches!(delete(&conn, id), Err(AppError::NotFound(_))));
    }

    #[test]
    fn media_type_from_extension() {
        assert_eq!(MediaType::from_extension("JPG"), Some(MediaType::Image));
        assert_eq!(MediaType::from_extension("mp4"), Some(MediaType::Video));
        assert_eq!(MediaType::from_extension("txt"), None);
    }
}
