//! Column readers that tolerate SQLite's loose typing. The Python app wrote numbers as TEXT in
//! some columns (e.g. `activities.elevation_gain` was a TEXT column before migration 44), so
//! numeric reads accept integers, reals and numeric strings alike.

use rusqlite::{Row, types::ValueRef};

pub fn opt_f64(row: &Row, column: &str) -> rusqlite::Result<Option<f64>> {
    Ok(match row.get_ref(column)? {
        ValueRef::Integer(i) => Some(i as f64),
        ValueRef::Real(f) => Some(f),
        ValueRef::Text(t) => std::str::from_utf8(t)
            .ok()
            .and_then(|s| s.trim().parse().ok()),
        ValueRef::Null | ValueRef::Blob(_) => None,
    })
}

pub fn opt_i64(row: &Row, column: &str) -> rusqlite::Result<Option<i64>> {
    Ok(opt_f64(row, column)?.map(|f| f.round() as i64))
}

pub fn opt_string(row: &Row, column: &str) -> rusqlite::Result<Option<String>> {
    Ok(match row.get_ref(column)? {
        ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Integer(i) => Some(i.to_string()),
        ValueRef::Real(f) => Some(f.to_string()),
        ValueRef::Null | ValueRef::Blob(_) => None,
    })
}

/// SQLite BOOLEAN columns hold 0/1, but older rows may be NULL or text.
pub fn bool_or(row: &Row, column: &str, default: bool) -> rusqlite::Result<bool> {
    Ok(opt_f64(row, column)?.map_or(default, |v| v != 0.0))
}
