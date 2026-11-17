//! System settings in `~/RunningData/appdata/settings.json`, shared with the Python app
//! (`processing/system_settings.py`). Keys this app doesn't know are kept when saving.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use ts_rs::TS;

use crate::{error::AppResult, paths};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct SystemSettings {
    /// "en", "de" or "fr".
    pub language: String,
    /// Only "metric" is supported.
    pub measures: String,
    /// "system", "light" or "dark".
    pub layout: String,
    /// Writes activity summaries with a local Ollama model.
    pub ai_enabled: bool,
    /// Base URL of the Ollama server.
    pub ai_url: String,
    /// Ollama model name, e.g. "qwen2.5:7b"; empty until one is chosen.
    pub ai_model: String,
    /// Weeks before an activity it is compared with; one of [`AI_PERIOD_WEEKS`].
    pub ai_period_weeks: u8,
    /// Boxes of the activity panel left out, per sport.
    pub activity_panel_hidden: PanelHidden,
    /// Looks up place names of activity starts at nominatim.openstreetmap.org.
    pub location_lookup: bool,
    /// How far photos copied into the media folder are scaled down.
    pub photo_size: PhotoSize,
}

/// Size of the photos copied into the media folder; linked originals are never changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PhotoSize {
    /// Kept as they are.
    Original,
    /// Longest side 3840 px, JPEG quality 90.
    Large,
    /// Longest side 2560 px, JPEG quality 85.
    #[default]
    Recommended,
    /// Longest side 1920 px, JPEG quality 80.
    Small,
}

impl PhotoSize {
    /// Longest side in pixels and JPEG quality (1-100), or `None` to keep photos as they are.
    pub fn limits(self) -> Option<(u32, u8)> {
        match self {
            Self::Original => None,
            Self::Large => Some((3840, 90)),
            Self::Recommended => Some((2560, 85)),
            Self::Small => Some((1920, 80)),
        }
    }
}

/// A box of the activity panel beside the activity list and in the detail window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PanelBox {
    /// The route shape in the overview.
    Map,
    /// Garmin Connect link in the overview.
    Links,
    /// Goal star, training session and segment trophy.
    Achievements,
    /// Photos, comment and AI summary at a glance.
    Notes,
    Weather,
    /// Best segments.
    Best,
}

/// Hidden panel boxes of runs, rides and walks; other activities show every box.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct PanelHidden {
    pub running: Vec<PanelBox>,
    pub cycling: Vec<PanelBox>,
    pub walking: Vec<PanelBox>,
}

pub const AI_PERIOD_WEEKS: [u8; 4] = [2, 4, 8, 12];

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            measures: "metric".into(),
            layout: "system".into(),
            ai_enabled: false,
            ai_url: "http://localhost:11434".into(),
            ai_model: String::new(),
            ai_period_weeks: 4,
            activity_panel_hidden: PanelHidden::default(),
            location_lookup: true,
            photo_size: PhotoSize::default(),
        }
    }
}

fn settings_file() -> AppResult<std::path::PathBuf> {
    Ok(paths::appdata_dir()?.join("settings.json"))
}

fn read_map(path: &Path) -> AppResult<Map<String, Value>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e.into()),
    }
}

pub fn load_from(path: &Path) -> AppResult<SystemSettings> {
    Ok(serde_json::from_value(Value::Object(read_map(path)?))?)
}

pub fn save_to(path: &Path, settings: &SystemSettings) -> AppResult<()> {
    let mut map = read_map(path)?;
    if let Value::Object(new) = serde_json::to_value(settings)? {
        map.extend(new);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_string(&map)?)?;
    Ok(())
}

pub fn load() -> AppResult<SystemSettings> {
    load_from(&settings_file()?)
}

pub fn save(settings: &SystemSettings) -> AppResult<()> {
    let valid = ["en", "de", "fr"].contains(&settings.language.as_str())
        && settings.measures == "metric"
        && ["system", "light", "dark"].contains(&settings.layout.as_str())
        && (settings.ai_url.starts_with("http://") || settings.ai_url.starts_with("https://"))
        && AI_PERIOD_WEEKS.contains(&settings.ai_period_weeks);
    if !valid {
        return Err(crate::error::AppError::Invalid(format!(
            "unsupported settings: {settings:?}"
        )));
    }
    save_to(&settings_file()?, settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_missing_and_keeps_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appdata/settings.json");
        assert_eq!(load_from(&path).unwrap(), SystemSettings::default());

        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, r#"{"language": "de", "extra": 1}"#).unwrap();
        let mut settings = load_from(&path).unwrap();
        assert_eq!(settings.language, "de");
        assert_eq!(settings.layout, "system");

        settings.layout = "dark".into();
        save_to(&path, &settings).unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["layout"], "dark");
        assert_eq!(saved["extra"], 1);
        assert_eq!(saved["ai_enabled"], false);
        assert_eq!(saved["photo_size"], "recommended");
    }

    #[test]
    fn photo_size_defaults_to_recommended_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"language": "de"}"#).unwrap();
        assert_eq!(load_from(&path).unwrap().photo_size, PhotoSize::Recommended);
        fs::write(&path, r#"{"photo_size": "small"}"#).unwrap();
        let settings = load_from(&path).unwrap();
        assert_eq!(settings.photo_size, PhotoSize::Small);
        assert_eq!(settings.photo_size.limits(), Some((1920, 80)));
        assert_eq!(PhotoSize::Original.limits(), None);
    }

    #[test]
    fn hidden_panel_boxes_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"activity_panel_hidden": {"cycling": ["weather", "best"]}}"#,
        )
        .unwrap();
        let settings = load_from(&path).unwrap();
        assert_eq!(
            settings.activity_panel_hidden.cycling,
            [PanelBox::Weather, PanelBox::Best]
        );
        assert!(settings.activity_panel_hidden.running.is_empty());
        save_to(&path, &settings).unwrap();
        assert_eq!(load_from(&path).unwrap(), settings);
    }
}
