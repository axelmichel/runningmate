//! SQLite access. The database file is shared with the Python app, so the schema and the meaning
//! of every column follow `database/database_handler.py`.

pub mod activities;
pub mod ai_summaries;
pub mod connectors;
pub mod media;
pub mod migrations;
pub mod performances;
pub mod route_segments;
pub mod users;
pub mod value;
pub mod weather;

use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// File name of the database inside the data folder, shared with the Python app.
pub const DB_FILE_NAME: &str = "running_data.db";

/// The app's single database connection, shared by all commands.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Opens (or creates) the database at `path` and brings the schema up to date.
    pub fn open(path: &Path) -> AppResult<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut conn: Connection) -> AppResult<Self> {
        // rusqlite's bundled SQLite enables foreign keys by default; Python's sqlite3 doesn't,
        // and the shared database has rows that would violate them. Match the Python app.
        conn.pragma_update(None, "foreign_keys", false)?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| AppError::DbPoisoned)
    }
}

/// Highest migration version recorded in `schema_version`, or 0 for a new database.
pub fn current_version(conn: &Connection) -> AppResult<i64> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
        [],
    )?;
    let version: Option<i64> =
        conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })?;
    Ok(version.unwrap_or(0))
}

/// Applies every migration above the current version, each in its own transaction.
///
/// Unlike the Python app, which logs a failed migration and carries on with a half-migrated
/// schema, a failure here rolls that migration back and is returned as an error.
pub fn migrate(conn: &mut Connection) -> AppResult<()> {
    let current = current_version(conn)?;
    for (version, statements) in migrations::MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        for statement in *statements {
            tx.execute_batch(statement)
                .map_err(|source| AppError::Migration {
                    version: *version,
                    source,
                })?;
        }
        tx.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [version],
        )?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn fresh_database_matches_python_schema() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();

        assert_eq!(current_version(&conn).unwrap(), 60);
        assert_eq!(
            columns(&conn, "activities"),
            [
                "id",
                "distance",
                "activity_type",
                "comment",
                "title",
                "duration",
                "date",
                "calories",
                "file_id",
                "elevation_gain",
                "edited",
                "new",
                "type_detail"
            ]
        );
        assert_eq!(
            columns(&conn, "runs"),
            [
                "id",
                "elevation_gain",
                "avg_speed",
                "avg_steps",
                "total_steps",
                "avg_power",
                "avg_heart_rate",
                "avg_pace",
                "fastest_pace",
                "slowest_pace",
                "pause",
                "track_img",
                "elevation_img",
                "map_html",
                "activity_id",
                "shoe_id"
            ]
        );
        assert!(columns(&conn, "walking").contains(&"shoe_id".to_string()));
        assert!(columns(&conn, "cycling").contains(&"bike_id".to_string()));
        assert_eq!(
            columns(&conn, "goals"),
            ["id", "name", "goal_type", "activity_types", "parameters"]
        );
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        migrate(&mut conn).unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows as usize, migrations::MIGRATIONS.len());
    }
}

/// Opt-in check against a copy of a real database:
/// `RUNNINGMATE_TEST_DB=/path/to/copy.db cargo test real_database -- --ignored`
#[cfg(test)]
mod real_database {
    use super::*;
    use crate::{
        activity_type::ViewMode,
        db::activities::{ActivityFilter, ActivityQuery, SortDirection, SortField},
    };

    #[test]
    #[ignore = "needs RUNNINGMATE_TEST_DB pointing at a copy of running_data.db"]
    fn reads_everything_from_a_real_database() {
        let path = std::env::var("RUNNINGMATE_TEST_DB").expect("RUNNINGMATE_TEST_DB");
        let db = Db::open(Path::new(&path)).unwrap();
        let conn = db.conn().unwrap();
        println!("schema version {}", current_version(&conn).unwrap());

        for view in [
            ViewMode::All,
            ViewMode::Running,
            ViewMode::Cycling,
            ViewMode::Walking,
        ] {
            let page = activities::list(
                &conn,
                &ActivityQuery {
                    view_mode: view,
                    offset: 0,
                    limit: 25,
                    sort_field: SortField::AvgPace,
                    sort_direction: SortDirection::Asc,
                    filter: ActivityFilter::default(),
                },
            )
            .unwrap();
            let summary = activities::summary(&conn, view, &ActivityFilter::default()).unwrap();
            assert_eq!(summary.count, page.total);
            println!(
                "{view:?}: {} activities, {:.1} km, {} h, {:.0} m",
                summary.count,
                summary.distance,
                summary.duration / 3600,
                summary.elevation_gain
            );
            let _ = performances::top(&conn, view).unwrap();
        }

        let ids: Vec<i64> = conn
            .prepare("SELECT id FROM activities")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for id in &ids {
            activities::get(&conn, *id).unwrap_or_else(|e| panic!("activity {id}: {e}"));
            media::list(&conn, *id, Path::new("/nonexistent-media")).unwrap();
            weather::get(&conn, *id).unwrap();
            performances::for_activity(&conn, *id).unwrap();
        }
        println!("decoded {} activities", ids.len());

        let started = std::time::Instant::now();
        let mut scores = [0usize; 13];
        for id in &ids {
            crate::analysis::segments(&conn, *id).unwrap();
            let effect = crate::analysis::training_effect(&conn, *id)
                .unwrap_or_else(|e| panic!("training effect {id}: {e}"));
            scores[effect.score as usize] += 1;
            crate::analysis::fitness_trend(&conn, *id).unwrap();
            crate::analysis::heart_rate_trend(&conn, *id).unwrap();
        }
        println!(
            "analysed {} activities in {:?}; training effect scores 1-12: {:?}",
            ids.len(),
            started.elapsed(),
            &scores[1..]
        );

        // Optionally also outline the newest routes from the archives (read-only).
        if let Ok(imports) = std::env::var("RUNNINGMATE_TEST_IMPORTS") {
            let started = std::time::Instant::now();
            let mut outlined = 0;
            for id in ids.iter().rev().take(50) {
                let route =
                    crate::importer::route_outline(&conn, *id, Path::new(&imports)).unwrap();
                outlined += usize::from(route.is_some());
                let zones =
                    crate::analysis::heart_rate_zones(&conn, *id, Path::new(&imports)).unwrap();
                assert!(zones.is_some_and(|z| z.len() == 5));
            }
            println!(
                "outlined {outlined}/50 routes in {:?} ({:?} each)",
                started.elapsed(),
                started.elapsed() / 50
            );
        }

        println!("user: {:?}", users::get(&conn).unwrap().map(|u| u.zones));
        let now = chrono::Utc::now().timestamp();
        for days in [90, 365] {
            let report = crate::analysis::health::report(&conn, now - days * 86_400, now).unwrap();
            println!(
                "health {days} days: {} activities, {} with heart rate, peak {:?}, last {:?}, {:?}",
                report.activities,
                report.heart_rate.len(),
                report.fitness_peak,
                report.fitness.last(),
                report.cardio
            );
        }
        for kind in [users::EquipmentKind::Shoe, users::EquipmentKind::Bike] {
            println!(
                "{kind:?}: {}",
                users::list_equipment(&conn, kind).unwrap().len()
            );
        }
    }
}
