//! Routes along OpenStreetMap paths from FOSSGIS's public OSRM servers
//! (routing.openstreetmap.de), for snapping drawn routes in the route fix editor. Only the
//! clicked points are sent, and only while snapping is switched on.

use std::time::Duration;

use serde::Deserialize;

use crate::{
    activity_type::ViewMode,
    error::{AppError, AppResult},
};

const SERVER: &str = "https://routing.openstreetmap.de";
/// More points per request are refused by the server.
const MAX_POINTS: usize = 100;

#[derive(Deserialize)]
struct Response {
    code: String,
    #[serde(default)]
    routes: Vec<Route>,
}

#[derive(Deserialize)]
struct Route {
    geometry: Geometry,
}

#[derive(Deserialize)]
struct Geometry {
    /// `[longitude, latitude]` pairs.
    coordinates: Vec<(f64, f64)>,
}

/// Foot paths for running and walking, bike paths for cycling. The servers expect "driving"
/// in the URL whatever the profile.
fn url(view: ViewMode, points: &[(f64, f64)]) -> String {
    let profile = if view == ViewMode::Cycling {
        "bike"
    } else {
        "foot"
    };
    let coordinates: Vec<String> = points
        .iter()
        .map(|(lat, lon)| format!("{lon:.6},{lat:.6}"))
        .collect();
    format!(
        "{SERVER}/routed-{profile}/route/v1/driving/{}?overview=full&geometries=geojson",
        coordinates.join(";")
    )
}

fn parse(json: &str) -> AppResult<Vec<(f64, f64)>> {
    let response: Response = serde_json::from_str(json)?;
    let route = response
        .routes
        .into_iter()
        .next()
        .filter(|_| response.code == "Ok")
        .ok_or_else(|| AppError::Invalid(format!("no route along paths ({})", response.code)))?;
    Ok(route
        .geometry
        .coordinates
        .into_iter()
        .map(|(lon, lat)| (lat, lon))
        .collect())
}

/// `[latitude, longitude]` route through `points` along paths.
pub async fn route(view: ViewMode, points: &[(f64, f64)]) -> AppResult<Vec<(f64, f64)>> {
    if points.len() < 2 || points.len() > MAX_POINTS {
        return Err(AppError::Invalid(format!(
            "a route along paths needs 2 to {MAX_POINTS} points"
        )));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("RunningMate/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::Invalid(format!("routing: {e}")))?;
    let text = client
        .get(url(view, points))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| AppError::Invalid(format!("routing service: {e}")))?
        .text()
        .await
        .map_err(|e| AppError::Invalid(format!("routing service: {e}")))?;
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_url_per_sport() {
        let points = [(52.5163, 13.3777), (52.517, 13.3889)];
        assert_eq!(
            url(ViewMode::Running, &points),
            "https://routing.openstreetmap.de/routed-foot/route/v1/driving/13.377700,52.516300;13.388900,52.517000?overview=full&geometries=geojson"
        );
        assert!(url(ViewMode::Cycling, &points).contains("/routed-bike/"));
    }

    #[test]
    fn reads_the_route_as_latitude_longitude() {
        let json = r#"{"code":"Ok","routes":[{"geometry":{"coordinates":[[13.3777,52.5163],[13.38,52.517]],"type":"LineString"}}]}"#;
        assert_eq!(parse(json).unwrap(), [(52.5163, 13.3777), (52.517, 13.38)]);
    }

    #[test]
    fn reports_no_route() {
        assert!(parse(r#"{"code":"NoRoute","routes":[]}"#).is_err());
    }
}
