//! Weather from Open-Meteo, ported from `processing/weather.py`.
//!
//! Python used the archive API for any past date and a current-conditions snapshot for today.
//! The archive lags a few days behind and returns empty values for recent dates, so dates within
//! the last 60 days use the forecast API's daily values instead (it covers recent past days).

use std::time::Duration;

use chrono::{Local, NaiveDate};
use serde::Deserialize;

use crate::db::weather::Weather;

const ARCHIVE_URL: &str = "https://archive-api.open-meteo.com/v1/archive";
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const DAILY: &str =
    "temperature_2m_max,temperature_2m_min,precipitation_sum,windspeed_10m_max,weather_code";
const RECENT_DAYS: i64 = 60;
const ATTEMPTS: u32 = 3;

#[derive(Debug, Deserialize)]
struct Response {
    daily: Option<Daily>,
}

#[derive(Debug, Deserialize)]
struct Daily {
    temperature_2m_max: Vec<Option<f64>>,
    temperature_2m_min: Vec<Option<f64>>,
    precipitation_sum: Vec<Option<f64>>,
    windspeed_10m_max: Vec<Option<f64>>,
    weather_code: Vec<Option<i64>>,
}

/// The API base URL for a date ("YYYY-MM-DD") relative to `today`.
fn endpoint(date: NaiveDate, today: NaiveDate) -> &'static str {
    if (today - date).num_days() <= RECENT_DAYS {
        FORECAST_URL
    } else {
        ARCHIVE_URL
    }
}

fn parse(body: &str) -> Option<Weather> {
    let daily = serde_json::from_str::<Response>(body).ok()?.daily?;
    let first = |v: &[Option<f64>]| v.first().copied().flatten();
    let weather = Weather {
        max_temp: first(&daily.temperature_2m_max),
        min_temp: first(&daily.temperature_2m_min),
        precipitation: first(&daily.precipitation_sum),
        max_wind_speed: first(&daily.windspeed_10m_max),
        weather_code: daily.weather_code.first().copied().flatten(),
    };
    // An all-empty day (data not available yet) isn't worth storing.
    (weather.max_temp.is_some() || weather.weather_code.is_some()).then_some(weather)
}

/// Daily weather at a place on a date, or `None` if it can't be fetched. Retries with backoff
/// like the Python service.
pub async fn fetch(latitude: f64, longitude: f64, date: &str) -> Option<Weather> {
    let day = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let url = endpoint(day, Local::now().date_naive());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    for attempt in 0..ATTEMPTS {
        let response = client
            .get(url)
            .query(&[
                ("latitude", latitude.to_string()),
                ("longitude", longitude.to_string()),
                ("start_date", date.to_string()),
                ("end_date", date.to_string()),
                ("daily", DAILY.to_string()),
                ("timezone", "auto".to_string()),
            ])
            .send()
            .await
            .and_then(|r| r.error_for_status());
        match response {
            Ok(response) => return parse(&response.text().await.ok()?),
            Err(_) if attempt + 1 < ATTEMPTS => {
                tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt))).await;
            }
            Err(_) => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_forecast_for_recent_days() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        assert_eq!(endpoint(today, today), FORECAST_URL);
        assert_eq!(
            endpoint(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(), today),
            FORECAST_URL
        );
        assert_eq!(
            endpoint(NaiveDate::from_ymd_opt(2025, 5, 6).unwrap(), today),
            ARCHIVE_URL
        );
    }

    #[test]
    fn parses_daily_values() {
        let body = r#"{"daily":{"time":["2025-05-06"],"temperature_2m_max":[12.6],
            "temperature_2m_min":[8.1],"precipitation_sum":[0.0],"windspeed_10m_max":[23.5],
            "weather_code":[3]}}"#;
        let weather = parse(body).unwrap();
        assert_eq!(weather.max_temp, Some(12.6));
        assert_eq!(weather.weather_code, Some(3));
        let empty = r#"{"daily":{"temperature_2m_max":[null],"temperature_2m_min":[null],
            "precipitation_sum":[null],"windspeed_10m_max":[null],"weather_code":[null]}}"#;
        assert_eq!(parse(empty), None);
    }
}
