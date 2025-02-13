import xml.etree.ElementTree as ET

import pandas as pd


def extract_activity_type(root, namespaces):
    """Extracts the activity type (Running, Biking, Walking) from a TCX file."""
    activity = root.find(".//tcx:Activity", namespaces)
    if activity is not None:
        return activity.attrib.get(
            "Sport", "Unknown"
        )  # Default to "Unknown" if missing

    return "Unknown"


def parse_tcx(file_path):
    """Reads a TCX file and extracts GPS coordinates, elevation, heart rate, and other metrics."""
    tree = ET.parse(file_path)
    root = tree.getroot()
    namespaces = {"tcx": "http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2"}
    trackpoints = []
    heart_rates = []
    for trackpoint in root.findall(".//tcx:Trackpoint", namespaces):
        lat = trackpoint.find(".//tcx:LatitudeDegrees", namespaces)
        lon = trackpoint.find(".//tcx:LongitudeDegrees", namespaces)
        ele = trackpoint.find(".//tcx:AltitudeMeters", namespaces)
        time = trackpoint.find(".//tcx:Time", namespaces)
        heart_rate = trackpoint.find(".//tcx:HeartRateBpm/tcx:Value", namespaces)
        extensions = trackpoint.find(".//tcx:Extensions", namespaces)
        if heart_rate is not None:
            heart_rates.append(int(heart_rate.text))
        steps = None
        power = None
        if extensions is not None:
            tpx = extensions.find(
                ".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}TPX"
            )
            if tpx is not None:
                steps = tpx.find(
                    ".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}RunCadence"
                )
                power = tpx.find(
                    ".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}Watts"
                )  # Schritte pro Minute

        if lat is not None and lon is not None and ele is not None and time is not None:
            trackpoints.append(
                (
                    time.text,
                    float(lat.text),
                    float(lon.text),
                    float(ele.text),
                    int(steps.text) if steps is not None else None,
                    int(power.text) if power is not None else None,
                )
            )

    df = pd.DataFrame(
        trackpoints,
        columns=["Time", "Latitude", "Longitude", "Elevation", "Steps", "Power"],
    )
    df["HeartRate"] = (
        pd.Series(heart_rates) if heart_rates else pd.Series(dtype="float")
    )
    activity_type = extract_activity_type(root, namespaces)
    return df, activity_type
