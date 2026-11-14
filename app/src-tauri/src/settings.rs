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
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            measures: "metric".into(),
            layout: "system".into(),
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
    }
}
