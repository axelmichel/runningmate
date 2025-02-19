import xml.etree.ElementTree as ET

import pandas as pd
from geopy.distance import geodesic

from processing.system_settings import ViewMode, mapActivityTypes
from utils.save_round import safe_round


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


def compute_averages(segment_data):
    avg_heart_rate = (
        sum(segment_data["heart_rate"]) / len(segment_data["heart_rate"])
        if segment_data["heart_rate"]
        else None
    )
    avg_power = (
        sum(segment_data["power"]) / len(segment_data["power"])
        if segment_data["power"]
        else None
    )
    avg_speed = (
        sum(segment_data["speed"]) / len(segment_data["speed"])
        if segment_data["speed"]
        else None
    )
    avg_steps = (
        sum(segment_data["steps"]) / len(segment_data["steps"])
        if segment_data["steps"]
        else None
    )
    avg_pace = avg_speed * 3.6 if avg_speed else None  # Convert m/s to km/h

    return {
        "avg_heart_rate": safe_round(avg_heart_rate),
        "avg_power": safe_round(avg_power),
        "avg_speed": avg_speed,
        "avg_pace": avg_pace,
        "avg_steps": safe_round(avg_steps) if avg_steps else None,
    }


def store_segment(segments, segment_data, time, activity_type):
    """Stores segment data and resets tracking variables."""
    if segment_data["seg_latitude"] is None:
        return  # Ignore empty segments

    # Compute averages
    averages = compute_averages(segment_data)

    # Store segment data
    segments.append(
        {
            "seg_latitude": segment_data["seg_latitude"],
            "seg_longitude": segment_data["seg_longitude"],
            "seg_avg_heart_rate": averages["avg_heart_rate"],
            "seg_avg_power": averages["avg_power"],
            "seg_avg_speed": averages["avg_speed"],
            "seg_avg_pace": averages["avg_pace"],
            "seg_avg_steps": (
                None if activity_type == ViewMode.CYCLE else averages["avg_steps"]
            ),
            "seg_distance": segment_data["seg_distance"],
            "seg_time_start": (
                str(segment_data["seg_time_start"])
                if segment_data["seg_time_start"]
                else None
            ),
            "seg_time_end": str(time) if time else None,
        }
    )

    # Reset segment data
    return get_segment_data()


def get_speed(row, df):
    speed = row.get("Speed")
    if speed is None and "DistDiff" in df and "TimeDiff" in df:
        speed = row["DistDiff"] / row["TimeDiff"] if row["TimeDiff"] > 0 else None
    return speed


def get_segment_data():
    return {
        "heart_rate": [],
        "power": [],
        "speed": [],
        "steps": [],
        "seg_latitude": None,
        "seg_longitude": None,
        "seg_distance": 0.0,
        "seg_time_start": None,
        "seg_time_end": None,
    }


def parse_segments(df, activity_type):
    """Splits activity data into distance-based segments (1KM for running/walking, 5KM for cycling)."""

    type = mapActivityTypes(activity_type)
    segment_length = 5.0 if type == ViewMode.CYCLE else 1.0  # Define segment length

    segments = []
    current_distance = 0.0
    segment_data = get_segment_data()

    prev_point = None  # To calculate distances

    for _index, row in df.iterrows():
        lat, lon, time = row["Latitude"], row["Longitude"], row["Time"]
        speed = get_speed(row, df)
        power = row.get("Power")
        heart_rate = row.get("HeartRate")
        steps = row.get("Steps")
        distance = 0.0

        # Set segment start point
        if segment_data["seg_latitude"] is None:
            (
                segment_data["seg_latitude"],
                segment_data["seg_longitude"],
                segment_data["seg_time_start"],
            ) = (lat, lon, time)

        # Compute distance from previous point
        if prev_point:
            distance = geodesic((prev_point[0], prev_point[1]), (lat, lon)).km
            current_distance += distance
        prev_point = (lat, lon)

        # Append data to segment
        if heart_rate:
            segment_data["heart_rate"].append(heart_rate)
        if power:
            segment_data["power"].append(power)
        if speed:
            segment_data["speed"].append(speed)
        if steps and type != ViewMode.CYCLE:
            segment_data["steps"].append(steps)
        segment_data["seg_distance"] += distance

        # Store full segment when distance threshold is met
        if current_distance >= segment_length:
            segment_data = store_segment(segments, segment_data, time, type)
            current_distance = 0.0  # Reset distance counter

    # Store the last incomplete segment if necessary
    if segment_data["seg_distance"] > 0:
        store_segment(segments, segment_data, time, type)

    return pd.DataFrame(segments)
