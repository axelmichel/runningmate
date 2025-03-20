from PyQt6.QtGui import QPalette
from PyQt6.QtWidgets import QApplication


class ViewMode:
    RUN = "Running"
    WALK = "Walking"
    CYCLE = "Cycling"
    ALL = "All"


class SortOrder:
    ASC = "ASC"
    DESC = "DESC"


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
