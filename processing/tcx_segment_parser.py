import pandas as pd
from geopy.distance import geodesic

from processing.system_settings import ViewMode, mapActivityTypes
from utils.save_avg import safe_avg
from utils.save_round import safe_round


class TcxSegmentParser:
    """
    A static class for parsing TCX files and extracting activity data.
    """

    def parse_segments(df: pd.DataFrame, activity_type: str) -> pd.DataFrame:
        """
        Splits activity data into distance-based segments.

        :param df: The input dataframe containing activity data.
        :param activity_type: The type of activity.
        :return: A dataframe containing segmented activity data.
        """
        type_group = mapActivityTypes(activity_type)
        segment_length = TcxSegmentParser._get_segment_length(type_group)
        segments = []
        current_distance = 0.0
        segment_data = TcxSegmentParser._get_segment_data()

        prev_point = None
        prev_elevation = None
        current_time = None

        for _, row in df.iterrows():
            current_time = row["Time"]  # Ensure this is always updated

            # Update segment data
            segment_data, prev_point, prev_elevation = (
                TcxSegmentParser._update_segment_data(
                    segment_data, row.to_dict(), prev_point, prev_elevation
                )
            )
            dist_diff = row.get("DistDiff", 0.0) / 1000.0  # ✅ FIXED: Now it's in KM!
            current_distance += dist_diff  # Add only the new di

            # Store segment if distance threshold is reached
            if current_distance >= segment_length:
                segment_data = TcxSegmentParser._store_segment(
                    segments, segment_data, current_time, type_group
                )
                current_distance = 0.0  # Reset distance

        # Store the last segment if there's remaining data
        if segment_data["seg_distance"] > 0:
            TcxSegmentParser._store_segment(
                segments,
                segment_data,
                current_time,
                type_group,  # Use the last updated time
            )

        print(f"Segmented {activity_type} activity into {len(segments)} segments")
        print(f"Segments: {segments}")
        return pd.DataFrame(segments)

    @staticmethod
    def _get_segment_data() -> dict:
        """
        Initializes a new segment data dictionary.

        :returns: A dictionary containing segment tracking data.
        """
        return {
            "heart_rate": [],
            "power": [],
            "speed": [],
            "steps": [],
            "elevation": [],
            "seg_latitude": None,
            "seg_longitude": None,
            "seg_distance": 0.0,
            "seg_elevation_gain": 0.0,
            "seg_time_start": None,
            "seg_time_end": None,
        }

    @staticmethod
    def _compute_averages(segment_data: dict) -> dict:
        """
        Computes the average values for various metrics in a segment.

        :param segment_data: The segment data dictionary.
        :return: dictionary containing average values for the segment.
        """

        avg_heart_rate = safe_avg(segment_data["heart_rate"])
        avg_power = safe_avg(segment_data["power"])
        avg_speed = safe_avg(segment_data["speed"])
        avg_steps = safe_avg(segment_data["steps"])
        avg_elevation = safe_avg(segment_data["elevation"])
        avg_pace = avg_speed * 3.6 if avg_speed else 0.0

        computed_values = {
            "avg_heart_rate": safe_round(avg_heart_rate),
            "avg_power": safe_round(avg_power),
            "avg_speed": avg_speed,
            "avg_pace": avg_pace,
            "avg_steps": safe_round(avg_steps) if avg_steps else 0,
            "avg_elevation": safe_round(avg_elevation) if avg_elevation else 0,
        }
        return computed_values

    @staticmethod
    def _store_segment(
        segments: list, segment_data: dict, time: str, activity_type: ViewMode
    ) -> dict | None:
        """
        Stores the current segment data and resets tracking variables.

        :param segments (list): List of stored segments.
        :param segment_data (dict): Current segment tracking data.
        :param time (str): End time of the segment.
        :param activity_type (ViewMode): The type of activity.
        :return dict: A new empty segment data dictionary.
        """
        if segment_data["seg_latitude"] is None:
            return None

        averages = TcxSegmentParser._compute_averages(segment_data)

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
                "seg_avg_elevation": averages["avg_elevation"],
                "seg_elevation_gain": segment_data["seg_elevation_gain"],
                "seg_distance": segment_data["seg_distance"],
                "seg_time_start": (
                    str(segment_data["seg_time_start"])
                    if segment_data["seg_time_start"]
                    else None
                ),
                "seg_time_end": str(time) if time else None,
            }
        )

        return TcxSegmentParser._get_segment_data()

    @staticmethod
    def _update_segment_data(
        segment_data: dict, row: dict, prev_point: tuple, prev_elevation: float
    ) -> tuple:
        """
        Updates segment data with current row information.

        :param segment_data (dict): Current segment tracking data.
        :param row (dict): The current row of data.
        :param prev_point (tuple): Previous GPS coordinates.
        :param prev_elevation (float): Previous elevation.
        :return tuple: Updated segment data, previous point, and previous elevation.
        """
        lat, lon, elevation, time = (
            row["Latitude"],
            row["Longitude"],
            row["Elevation"],
            row["Time"],
        )
        distance = geodesic(prev_point, (lat, lon)).km if prev_point else 0.0
        speed = TcxSegmentParser.get_speed(row, segment_data)

        if segment_data["seg_latitude"] is None:
            (
                segment_data["seg_latitude"],
                segment_data["seg_longitude"],
                segment_data["seg_time_start"],
            ) = (lat, lon, time)

        if "HeartRate" in row:
            segment_data["heart_rate"].append(row["HeartRate"])
        if "Power" in row:
            segment_data["power"].append(row["Power"])
        if "Steps" in row:
            segment_data["steps"].append(row["Steps"])

        if prev_elevation is not None and elevation > prev_elevation:
            segment_data["seg_elevation_gain"] += elevation - prev_elevation
        segment_data["speed"].append(speed)
        segment_data["elevation"].append(elevation)
        segment_data["seg_distance"] += distance
        segment_data["seg_time_end"] = time
        return segment_data, (lat, lon), elevation

    @staticmethod
    def _get_segment_length(type_group):
        """
        Defines the segment length based on activity type.

        :param type_group (ViewMode): The type of activity.
        :return float: The segment length in kilometers.
        """
        segment_length = 5.0 if type_group is ViewMode.CYCLE else 1.0
        return segment_length

    @staticmethod
    def get_speed(row, df):
        speed = row.get("Speed")
        if speed is None and "DistDiff" in df and "TimeDiff" in df:
            speed = row["DistDiff"] / row["TimeDiff"] if row["TimeDiff"] > 0 else None
        return speed
