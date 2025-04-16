import json
import os

from PyQt6.QtGui import QPalette
from PyQt6.QtWidgets import QApplication

APPDATA_DIR = os.path.expanduser("~/RunningData/appdata")
if not os.path.exists(APPDATA_DIR):
    os.makedirs(APPDATA_DIR)

SETTINGS_FILE = os.path.join(APPDATA_DIR, "settings.json")


class ViewMode:
    RUN = "Running"
    WALK = "Walking"
    CYCLE = "Cycling"
    ALL = "All"


class SortOrder:
    ASC = "ASC"
    DESC = "DESC"


def load_settings_config():
    """
    Load all stored sync dates from the JSON file.

    :return: dict, all sync dates (e.g., {'last_garmin_sync': '2025-02-26T10:30:00', 'last_icloud_sync': '2025-02-25T09:00:00'})
    """
    if os.path.exists(SETTINGS_FILE):
        with open(SETTINGS_FILE, "r") as f:
            return json.load(f)
    return {}  # Return an empty dictionary if file doesn't exist


def get_settings_value(key: str, default_value=None):
    """
    Get a specific setting value from the settings file.

    :param key: str, the key of the setting to retrieve
    :param default_value: any, the default value to return if the key is not found
    :return: any, the value of the setting or the default value
    """
    settings = load_settings_config()
    return settings.get(key, default_value)


def get_settings_locale(default_value="en_US.utf-8"):
    """
    Get a specific setting value from the settings file.

    :param default_value: any, the default value to return if the key is not found
    :return: any, the value of the setting or the default value
    """
    language = get_settings_value("language", "en")
    if language == "de":
        return "de_DE.utf-8"
    elif language == "fr":
        return "fr_FR.utf-8"
    return default_value


def mapActivityTypes(activity_type: str):
    """Maps activity type strings to ViewMode categories."""

    activity_map = {
        ViewMode.RUN: {"Running", "Trailrun", "Run", "Trackrun", "Track"},
        ViewMode.WALK: {"Walking", "Hike", "Trekking", "Other", "Nordic Walking"},
        ViewMode.CYCLE: {
            "Cycling",
            "Bike",
            "MTB",
            "Bicycle",
            "Biking",
            "E-Bike",
            "Gravelbike",
            "Mountainbike",
        },
    }

    for mode, valid_names in activity_map.items():
        if activity_type in valid_names:
            return mode

    return ViewMode.ALL


def get_type_details(view_type: ViewMode):
    """Returns a detailed description for the given view type."""

    if view_type == ViewMode.RUN:
        return [
            "Running",
            "Trail",
            "Track",
            "Endurance",
            "Interval",
            "Recovery",
            "Speed",
            "Long Run",
        ]
    if view_type == ViewMode.WALK:
        return ["Walking", "Hiking", "Nordic Walking", "Trekking", "Other"]
    if view_type == ViewMode.CYCLE:
        return [
            "Cycling",
            "Biking",
            "Mountain Biking",
            "Gravel Biking",
            "E-Bike",
            "Endurance",
            "Recovery",
        ]
    return "Unknown"


def getAllowedTypes(view_type: ViewMode):
    """Returns a list of allowed activity types for the given view type."""

    if view_type == ViewMode.RUN:
        return ["Running", "Trailrun", "Run", "Trackrun", "Track"]
    if view_type == ViewMode.WALK:
        return ["Walking", "Hike", "Trekking", "Other", "Nordic Walking"]
    if view_type == ViewMode.CYCLE:
        return [
            "Cycling",
            "Bike",
            "MTB",
            "Bicycle",
            "Biking",
            "E-Bike",
            "Gravelbike",
            "Mountainbike",
        ]

    return []


def get_system_background_color():
    """Fetches the system-defined window background color."""
    app = QApplication.instance() or QApplication([])
    palette = app.palette()
    color = palette.color(QPalette.ColorRole.Window)
    return color.name()
