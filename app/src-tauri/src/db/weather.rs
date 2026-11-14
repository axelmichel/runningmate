//! Weather per activity (`weather` table), filled from Open-Meteo after import.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::value::{opt_f64, opt_i64};
use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Weather {
    /// °C. For activities on the current day both temperatures hold the current temperature.
    pub max_temp: Option<f64>,
    pub min_temp: Option<f64>,
    /// mm.
    pub precipitation: Option<f64>,
    /// km/h.
    pub max_wind_speed: Option<f64>,
    /// WMO weather code.
    #[ts(type = "number | null")]
    pub weather_code: Option<i64>,
}

pub fn get(conn: &Connection, activity_id: i64) -> AppResult<Option<Weather>> {
    Ok(conn
        .query_row(
            "SELECT max_temp, min_temp, precipitation, max_wind_speed, weather_code
             FROM weather WHERE activity_id = ?1",
            [activity_id],
            |row| {
                Ok(Weather {
                    max_temp: opt_f64(row, "max_temp")?,
                    min_temp: opt_f64(row, "min_temp")?,
                    precipitation: opt_f64(row, "precipitation")?,
                    max_wind_speed: opt_f64(row, "max_wind_speed")?,
                    weather_code: opt_i64(row, "weather_code")?,
                })
            },
        )
        .optional()?)
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "used by the weather fetch after import (milestone 3)"
    )
)]
pub fn upsert(conn: &Connection, activity_id: i64, weather: &Weather) -> AppResult<()> {
    // `weather.activity_id` is a TEXT column in the Python schema.
    conn.execute(
        "INSERT INTO weather (activity_id, max_temp, min_temp, precipitation, max_wind_speed,
            weather_code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (activity_id) DO UPDATE SET max_temp = excluded.max_temp,
            min_temp = excluded.min_temp, precipitation = excluded.precipitation,
            max_wind_speed = excluded.max_wind_speed, weather_code = excluded.weather_code",
        params![
            activity_id.to_string(),
            weather.max_temp,
            weather.min_temp,
            weather.precipitation,
            weather.max_wind_speed,
            weather.weather_code
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    #[test]
    fn upserts_and_reads() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        assert_eq!(get(&conn, 1).unwrap(), None);

        let mut weather = Weather {
            max_temp: Some(12.3),
            min_temp: Some(5.4),
            precipitation: Some(0.3),
            max_wind_speed: Some(21.6),
            weather_code: Some(51),
        };
        upsert(&conn, 1, &weather).unwrap();
        weather.weather_code = Some(0);
        upsert(&conn, 1, &weather).unwrap();
        assert_eq!(get(&conn, 1).unwrap(), Some(weather));
    }
}
