//! Backend for the social share editor: map tiles and photos as bytes (the canvas can't load
//! them straight from other origins and still be exported), saving and the share menu.

use std::{path::PathBuf, time::Duration};

use base64::Engine;
use tauri::ipc::Response;

use crate::{
    db::media::MediaType,
    error::{AppError, AppResult},
    share,
};

const ESRI: &str = "https://server.arcgisonline.com/ArcGIS/rest/services/Canvas";
/// Tiles exist up to this zoom; the editor scales them beyond.
const MAX_ZOOM: u8 = 16;

/// One map tile as image bytes: the light or dark gray canvas, its base or its label layer
/// (the same tiles as `features/detail/basemap.tsx`).
#[tauri::command(async)]
pub async fn map_tile(dark: bool, labels: bool, z: u8, x: u32, y: u32) -> AppResult<Response> {
    if z > MAX_ZOOM || x >= 1 << z || y >= 1 << z {
        return Err(AppError::Invalid(format!("no map tile {z}/{x}/{y}")));
    }
    let theme = if dark { "Dark" } else { "Light" };
    let layer = if labels { "Reference" } else { "Base" };
    let url = format!("{ESRI}/World_{theme}_Gray_{layer}/MapServer/tile/{z}/{y}/{x}");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Invalid(format!("map tile {z}/{x}/{y}: {e}")))?;
    let bytes = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| AppError::Invalid(format!("map tile {z}/{x}/{y}: {e}")))?
        .bytes()
        .await
        .map_err(|e| AppError::Invalid(format!("map tile {z}/{x}/{y}: {e}")))?;
    Ok(Response::new(bytes.to_vec()))
}

/// An image file's bytes, for a background photo; other files are refused.
#[tauri::command(async)]
pub fn read_image(path: PathBuf) -> AppResult<Response> {
    let image = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(MediaType::from_extension)
        == Some(MediaType::Image);
    if !image {
        return Err(AppError::Invalid(format!(
            "not an image: {}",
            path.display()
        )));
    }
    Ok(Response::new(std::fs::read(&path)?))
}

fn decode(png: &str) -> AppResult<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(png)
        .map_err(|e| AppError::Invalid(format!("invalid image data: {e}")))
}

/// Writes the finished image (base64 PNG) to `path`.
#[tauri::command(async)]
pub fn save_share_image(path: PathBuf, png: String) -> AppResult<()> {
    std::fs::write(path, decode(&png)?)?;
    Ok(())
}

/// Opens the share menu for the finished image (base64 PNG), stored under `name` in a temporary
/// folder, next to `anchor` (the button: x, y, width, height).
#[tauri::command(async)]
pub fn share_image(
    window: tauri::WebviewWindow,
    name: String,
    png: String,
    anchor: [f64; 4],
) -> AppResult<()> {
    let dir = std::env::temp_dir().join("runningmate-share");
    std::fs::create_dir_all(&dir)?;
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let file = dir.join(format!("{}.png", safe.trim()));
    std::fs::write(&file, decode(&png)?)?;
    share::present(&window, file, anchor)
}
