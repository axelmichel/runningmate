//! Links between local activities and their copies in connected services (`connector_links`),
//! so activities aren't imported or uploaded twice.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::value::{opt_f64, opt_i64, opt_string};
use crate::error::AppResult;

pub const GARMIN: &str = "garmin";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Direction {
    /// The activity came from the service.
    Import,
    /// The activity was uploaded to the service.
    Export,
}

impl Direction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Export => "export",
        }
    }
}

pub fn link(
    conn: &Connection,
    activity_id: i64,
    connector: &str,
    remote_id: Option<&str>,
    direction: Direction,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO connector_links (activity_id, connector, remote_id, direction, linked_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (activity_id, connector) DO UPDATE SET remote_id = excluded.remote_id,
            direction = excluded.direction, linked_at = excluded.linked_at",
        params![
            activity_id,
            connector,
            remote_id,
            direction.as_str(),
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

/// A link between an activity and a service.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConnectorLink {
    pub connector: String,
    pub remote_id: Option<String>,
    pub direction: Direction,
}

pub fn links(conn: &Connection, activity_id: i64) -> AppResult<Vec<ConnectorLink>> {
    let mut stmt = conn.prepare(
        "SELECT connector, remote_id, direction FROM connector_links
         WHERE activity_id = ?1 ORDER BY connector",
    )?;
    let rows = stmt.query_map([activity_id], |row| {
        let direction: String = row.get("direction")?;
        Ok(ConnectorLink {
            connector: row.get("connector")?,
            remote_id: opt_string(row, "remote_id")?,
            direction: if direction == "import" {
                Direction::Import
            } else {
                Direction::Export
            },
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// An activity that could be uploaded to a service.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportCandidate {
    #[ts(type = "number")]
    pub id: i64,
    pub title: Option<String>,
    #[ts(type = "number | null")]
    pub date: Option<i64>,
    pub distance: Option<f64>,
}

/// Activities since `since` (Unix timestamp) with an archived file that aren't linked to the
/// connector yet. Files downloaded from Garmin by the Python app (`garmin_activity_…`) count as
/// linked even though that app recorded no link.
pub fn export_candidates(
    conn: &Connection,
    connector: &str,
    since: i64,
) -> AppResult<Vec<ExportCandidate>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, date, distance FROM activities
         WHERE date >= ?1 AND file_id IS NOT NULL AND file_id <> ''
           AND file_id NOT LIKE 'garmin\\_activity\\_%' ESCAPE '\\'
           AND id NOT IN (SELECT activity_id FROM connector_links WHERE connector = ?2)
         ORDER BY date",
    )?;
    let rows = stmt.query_map(params![since, connector], |row| {
        Ok(ExportCandidate {
            id: row.get("id")?,
            title: opt_string(row, "title")?,
            date: opt_i64(row, "date")?,
            distance: opt_f64(row, "distance")?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    #[test]
    fn candidates_skip_linked_and_garmin_files() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        for (id, file_id) in [
            (1, "2025-05-06_run_hk_1"),
            (2, "garmin_activity_2025-05-07_99"),
            (3, "2025-05-08_walk_hk_3"),
        ] {
            insert_activity(&conn, id, "Running", 1_746_500_000 + id * 86_400, 5.0, "x");
            conn.execute(
                "UPDATE activities SET file_id = ?1 WHERE id = ?2",
                params![file_id, id],
            )
            .unwrap();
        }
        link(&conn, 3, GARMIN, Some("555"), Direction::Export).unwrap();
        let ids: Vec<i64> = export_candidates(&conn, GARMIN, 0)
            .unwrap()
            .iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, [1]);
        assert_eq!(
            links(&conn, 3).unwrap(),
            [ConnectorLink {
                connector: GARMIN.into(),
                remote_id: Some("555".into()),
                direction: Direction::Export,
            }]
        );
        // Newer than all activities: nothing to upload.
        assert!(
            export_candidates(&conn, GARMIN, 2_000_000_000)
                .unwrap()
                .is_empty()
        );
    }
}
