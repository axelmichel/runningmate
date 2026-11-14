//! Grouping of raw activity type names into the app's views, ported from
//! `processing/system_settings.py` (`ViewMode`, `map_activity_types`, `get_type_details`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The views in the sidebar. Serialised with the Python `ViewMode` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ViewMode {
    All,
    Running,
    Cycling,
    Walking,
}

const RUN_TYPES: &[&str] = &["Running", "Trailrun", "Run", "Trackrun", "Track"];
const WALK_TYPES: &[&str] = &["Walking", "Hike", "Trekking", "Other", "Nordic Walking"];
const CYCLE_TYPES: &[&str] = &[
    "Cycling",
    "Bike",
    "MTB",
    "Bicycle",
    "Biking",
    "E-Bike",
    "Gravelbike",
    "Mountainbike",
];

impl ViewMode {
    /// View a raw `activities.activity_type` belongs to; unknown names map to `All`.
    pub fn from_activity_type(activity_type: &str) -> Self {
        [Self::Running, Self::Walking, Self::Cycling]
            .into_iter()
            .find(|mode| mode.allowed_types().contains(&activity_type))
            .unwrap_or(Self::All)
    }

    /// Raw activity type names grouped under this view (empty for `All`).
    pub fn allowed_types(self) -> &'static [&'static str] {
        match self {
            Self::Running => RUN_TYPES,
            Self::Walking => WALK_TYPES,
            Self::Cycling => CYCLE_TYPES,
            Self::All => &[],
        }
    }

    /// Choices offered for `activities.type_detail` on the edit page.
    pub fn type_details(self) -> &'static [&'static str] {
        match self {
            Self::Running => &[
                "Running",
                "Trail",
                "Track",
                "Endurance",
                "Interval",
                "Recovery",
                "Speed",
                "Long Run",
            ],
            Self::Walking => &["Walking", "Hiking", "Nordic Walking", "Trekking", "Other"],
            Self::Cycling => &[
                "Cycling",
                "Biking",
                "Mountain Biking",
                "Gravel Biking",
                "E-Bike",
                "Endurance",
                "Recovery",
            ],
            Self::All => &[],
        }
    }

    /// Per-type table holding the detailed metrics, `None` for `All`.
    pub fn table(self) -> Option<&'static str> {
        match self {
            Self::Running => Some("runs"),
            Self::Walking => Some("walking"),
            Self::Cycling => Some("cycling"),
            Self::All => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ViewMode;

    #[test]
    fn maps_raw_types_to_views() {
        assert_eq!(ViewMode::from_activity_type("Trailrun"), ViewMode::Running);
        assert_eq!(ViewMode::from_activity_type("Other"), ViewMode::Walking);
        assert_eq!(ViewMode::from_activity_type("E-Bike"), ViewMode::Cycling);
        assert_eq!(ViewMode::from_activity_type("Swimming"), ViewMode::All);
    }

    #[test]
    fn only_typed_views_have_a_table() {
        assert_eq!(ViewMode::Cycling.table(), Some("cycling"));
        assert_eq!(ViewMode::All.table(), None);
    }
}
