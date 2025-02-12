class ViewMode:
    RUN = "Running"
    WALK = "Walking"
    CYCLE = "Cycling"
    ALL = "All"

class SortOrder:
    ASC = "ASC"
    DESC = "DESC"
def mapActivityTypes(activity_type: str) -> str:
    """Maps activity type strings to ViewMode categories."""

    activity_map = {
        ViewMode.RUN: {"Running", "Trailrun", "Run", "Trackrun", "Track"},
        ViewMode.WALK: {"Walking", "Hike", "Trekking"},
        ViewMode.CYCLE: {"Cycling", "Bike", "MTB", "Roadbike", "Bicycle"}
    }

    for mode, valid_names in activity_map.items():
        if activity_type in valid_names:
            return mode

    return ViewMode.ALL