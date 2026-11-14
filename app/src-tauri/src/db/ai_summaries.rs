//! Activity summaries written by a local AI model (`ai_summaries`), so they are only generated
//! once per activity.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AiSummary {
    /// Ollama model that wrote the text.
    pub model: String,
    pub text: String,
    /// Unix timestamp.
    #[ts(type = "number")]
    pub created: i64,
}

pub fn get(conn: &Connection, activity_id: i64) -> AppResult<Option<AiSummary>> {
    Ok(conn
        .query_row(
            "SELECT model, text, created FROM ai_summaries WHERE activity_id = ?1",
            [activity_id],
            |row| {
                Ok(AiSummary {
                    model: row.get("model")?,
                    text: row.get("text")?,
                    created: row.get("created")?,
                })
            },
        )
        .optional()?)
}

pub fn upsert(conn: &Connection, activity_id: i64, summary: &AiSummary) -> AppResult<()> {
    conn.execute(
        "INSERT INTO ai_summaries (activity_id, model, text, created) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (activity_id) DO UPDATE SET model = excluded.model, text = excluded.text,
            created = excluded.created",
        params![activity_id, summary.model, summary.text, summary.created],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, activity_id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM ai_summaries WHERE activity_id = ?1",
        [activity_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn stores_replaces_and_deletes() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        assert_eq!(get(&conn, 1).unwrap(), None);

        let first = AiSummary {
            model: "qwen2.5:7b".into(),
            text: "Steady run.".into(),
            created: 100,
        };
        upsert(&conn, 1, &first).unwrap();
        assert_eq!(get(&conn, 1).unwrap(), Some(first.clone()));

        let second = AiSummary {
            text: "Faster than usual.".into(),
            created: 200,
            ..first
        };
        upsert(&conn, 1, &second).unwrap();
        assert_eq!(get(&conn, 1).unwrap(), Some(second));

        delete(&conn, 1).unwrap();
        assert_eq!(get(&conn, 1).unwrap(), None);
    }
}
