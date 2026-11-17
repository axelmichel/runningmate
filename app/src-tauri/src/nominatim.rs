//! Place names for activity starts from OpenStreetMap's Nominatim reverse geocoding
//! (nominatim.openstreetmap.org). Following its usage policy, requests identify the app, are
//! spaced at least a second apart across the whole app, and every result is stored, so each
//! activity is looked up once. Only the start point's coordinates are sent, and only while the
//! lookup is switched on in the settings.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Deserialize;
use serde_json::{Map, Value};

const URL: &str = "https://nominatim.openstreetmap.org/reverse";
/// Town and suburb level.
const ZOOM: &str = "14";
/// The policy allows one request per second; a little more leaves room for clock jitter.
pub const INTERVAL: Duration = Duration::from_millis(1100);
const USER_AGENT: &str = concat!(
    "RunningMate/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/axelmichel/runningmate)"
);

/// When the next request may start.
static NEXT_SLOT: Mutex<Option<Instant>> = Mutex::new(None);

/// Why a lookup gave no answer.
#[derive(Debug, PartialEq)]
pub enum LookupError {
    /// No connection, a timeout, rate limiting or a server error: worth trying again later.
    Unavailable(String),
    /// The server answered with something unreadable; trying again won't help.
    Failed(String),
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    address: Map<String, Value>,
    #[serde(default)]
    error: Option<String>,
}

/// Reserves the next request slot at or after `now` and moves `next` one interval on.
fn reserve(next: &mut Option<Instant>, now: Instant) -> Instant {
    let slot = next.map_or(now, |next| next.max(now));
    *next = Some(slot + INTERVAL);
    slot
}

/// Waits for this request's slot, so requests start at least [`INTERVAL`] apart.
async fn throttle() {
    let slot = {
        let mut next = NEXT_SLOT.lock().unwrap_or_else(|e| e.into_inner());
        reserve(&mut next, Instant::now())
    };
    tokio::time::sleep_until(tokio::time::Instant::from_std(slot)).await;
}

/// Town (or village) and, where there is one, the part of it: "Freiburg im Breisgau, Wiehre".
/// Outside any town the county or state is used.
fn place_name(address: &Map<String, Value>) -> Option<String> {
    let field = |keys: &[&str]| {
        keys.iter()
            .filter_map(|key| address.get(*key)?.as_str())
            .map(str::trim)
            .find(|value| !value.is_empty())
    };
    let Some(place) = field(&["city", "town", "village", "municipality", "hamlet"]) else {
        return field(&["county", "state_district", "state", "country"]).map(str::to_string);
    };
    let part = ["suburb", "city_district", "borough", "quarter", "hamlet"]
        .iter()
        .filter_map(|key| address.get(*key)?.as_str())
        .map(str::trim)
        .find(|value| !value.is_empty() && *value != place);
    Some(match part {
        Some(part) => format!("{place}, {part}"),
        None => place.to_string(),
    })
}

/// The place name in a response; `None` when Nominatim found nothing there (e.g. at sea).
fn parse(json: &str) -> Result<Option<String>, LookupError> {
    let response: Response =
        serde_json::from_str(json).map_err(|e| LookupError::Failed(format!("place name: {e}")))?;
    if response.error.is_some() {
        return Ok(None);
    }
    Ok(place_name(&response.address))
}

/// Place name of a point in `language` ("en", "de", …), throttled to the usage policy.
pub async fn lookup(
    latitude: f64,
    longitude: f64,
    language: &str,
) -> Result<Option<String>, LookupError> {
    let unavailable = |e: reqwest::Error| LookupError::Unavailable(format!("place names: {e}"));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(USER_AGENT)
        .build()
        .map_err(unavailable)?;
    throttle().await;
    let text = client
        .get(URL)
        .query(&[
            ("lat", format!("{latitude:.6}")),
            ("lon", format!("{longitude:.6}")),
            ("format", "jsonv2".to_string()),
            ("zoom", ZOOM.to_string()),
            ("addressdetails", "1".to_string()),
            ("accept-language", language.to_string()),
        ])
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(unavailable)?
        .text()
        .await
        .map_err(unavailable)?;
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_requests_an_interval_apart() {
        let start = Instant::now();
        let mut next = None;
        assert_eq!(reserve(&mut next, start), start);
        assert_eq!(reserve(&mut next, start), start + INTERVAL);
        assert_eq!(reserve(&mut next, start), start + INTERVAL * 2);
        // After a quiet spell the next request may go right away.
        let later = start + Duration::from_secs(60);
        assert_eq!(reserve(&mut next, later), later);
    }

    #[test]
    fn names_town_and_suburb() {
        let json = r#"{"place_id":123,"lat":"47.98","lon":"7.86","category":"place",
            "type":"suburb","place_rank":20,"addresstype":"suburb","name":"Wiehre",
            "display_name":"Wiehre, Freiburg im Breisgau, Baden-Württemberg, 79100, Deutschland",
            "address":{"suburb":"Wiehre","city":"Freiburg im Breisgau","state":"Baden-Württemberg",
            "ISO3166-2-lvl4":"DE-BW","postcode":"79100","country":"Deutschland","country_code":"de"}}"#;
        assert_eq!(
            parse(json).unwrap().as_deref(),
            Some("Freiburg im Breisgau, Wiehre")
        );
    }

    #[test]
    fn names_villages_and_the_countryside() {
        let village = r#"{"address":{"village":"Oberried","county":"Landkreis Breisgau-Hochschwarzwald","country":"Deutschland"}}"#;
        assert_eq!(parse(village).unwrap().as_deref(), Some("Oberried"));
        let hamlet = r#"{"address":{"hamlet":"Weilersbach","village":"Oberried"}}"#;
        assert_eq!(
            parse(hamlet).unwrap().as_deref(),
            Some("Oberried, Weilersbach")
        );
        let same = r#"{"address":{"city":"Berlin","suburb":"Berlin","borough":"Mitte"}}"#;
        assert_eq!(parse(same).unwrap().as_deref(), Some("Berlin, Mitte"));
        let forest = r#"{"address":{"county":"Landkreis Breisgau-Hochschwarzwald","state":"Baden-Württemberg"}}"#;
        assert_eq!(
            parse(forest).unwrap().as_deref(),
            Some("Landkreis Breisgau-Hochschwarzwald")
        );
    }

    #[test]
    fn nothing_found_and_broken_answers() {
        assert_eq!(parse(r#"{"error":"Unable to geocode"}"#), Ok(None));
        assert_eq!(parse(r#"{"address":{}}"#), Ok(None));
        assert!(matches!(parse("<html>"), Err(LookupError::Failed(_))));
    }
}
