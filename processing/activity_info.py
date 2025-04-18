import locale
import os
from datetime import datetime, timedelta
from typing import Any, Dict, Optional

import pandas as pd

from database.database_handler import DatabaseHandler
from processing.system_settings import (
    ViewMode,
    get_settings_locale,
    get_allowed_types,
    map_activity_types,
)


class ActivityInfo:
    """
    A class to retrieve and format activity details from an SQLite database,
    including track images, media files, and weather information.
    """

    def __init__(self, db_handler: DatabaseHandler, file_path: str):
        """
        Initialize the ActivityInfo class.
        :param db_handler: database.DatabaseHandler
        """
        self.conn = db_handler.conn
        self.file_path = file_path

    @staticmethod
    def format_date(timestamp: int) -> str:
        """
        Format the timestamp as a human-readable date string.

        - If the date is today: "Today HH:MM"
        - If the date is yesterday: "Yesterday HH:MM"
        - If the date is in the same week: "Weekday HH:MM" (e.g., "Friday 14:30")
        - Otherwise: "dd.MM.YYYY HH:MM"

        :param timestamp: int
            Unix timestamp representing the activity date.
        :return: str
            Formatted date string.
        """
        user_locale = get_settings_locale()
        locale.setlocale(locale.LC_TIME, user_locale)

        dt = datetime.fromtimestamp(timestamp)
        now = datetime.now()

        if dt.date() == now.date():
            return f"Today {dt.strftime('%H:%M')}"
        elif dt.date() == (now.date() - timedelta(days=1)):
            return f"Yesterday {dt.strftime('%H:%M')}"
        elif dt.date() >= (now.date() - timedelta(days=now.weekday())):
            return f"{dt.strftime('%A')} {dt.strftime('%H:%M')}"
        else:
            return dt.strftime("%a, %d.%m.%Y %H:%M")

    def get_latest_activity_id(
        self,
        activity_id: Optional[int] = None,
        activity_type: Optional[ViewMode] = None,
    ) -> Optional[int]:
        """
        Retrieves the latest activity ID based on the given parameters.

        - If `activity_id` is provided, it is used directly.
        - If `activity_type` is provided, it fetches the latest activity of that type.
        - If no parameters are provided, it fetches the latest overall activity.

        :param activity_id: Optional[int]
            The specific activity ID to use (if provided).
        :param activity_type: Optional[ViewMode]
            The type of activity to filter by (if provided).
        :return: Optional[int]
            The ID of the selected activity, or None if no activity is found.
        """
        if activity_id is not None:
            return activity_id

        query = "SELECT id FROM activities"
        params = []

        if activity_type is not None and activity_type != ViewMode.ALL:
            allowed_types = get_allowed_types(activity_type)
            placeholders = ", ".join("?" * len(allowed_types))  # Generate placeholders
            query += f" WHERE activity_type IN ({placeholders})"
            params = tuple(allowed_types)

        query += " ORDER BY date DESC LIMIT 1"

        result = pd.read_sql(query, self.conn, params=params)

        if not result.empty:
            return int(result.iloc[0]["id"])
        return None

    def get_activity_info(
        self,
        activity_type: Optional[ViewMode] = None,
        activity_id: Optional[int] = None,
    ) -> Optional[Dict[str, Any]]:
        """
        Retrieve and format activity details, selecting the latest activity if no ID is provided.

        - If `activity_type` is given, it fetches details for the latest activity of that type.
        - If `activity_id` is given, it fetches details for that activity.
        - If neither is provided, it fetches details for the latest activity overall.

        :param activity_id: Optional[int]
            The ID of the activity to fetch (if provided).
        :param activity_type: Optional[str]
            The type of activity to filter by when selecting the latest activity (if provided).
        :return: Optional[Dict[str, Any]]
            A dictionary containing formatted activity details, or None if no data is found.

            Example Output:
            {
                "id": 1234,
                "category": "RUN",
                "date": "Friday 14:30",
                "title": "Evening Run",
                "distance": 5.42,
                "duration": "00:30:15",
                "elevation_gain": 120,
                "track": "/home/user/RunningData/tracks/1234_track.png",
                "media": {"media_type": "image", "file_path": "/media/photos/run1.jpg"},
                "weather": {
                    "avg_temp": 15.5,
                    "precipitation": 2.3,
                    "max_wind_speed": 20,
                    "weather_code": "Partly Cloudy"
                },
                "extra": {
                    "avg_heartrate": 145,
                    "avg_pace": "05:30"
                }
            }
        """
        # Select the appropriate activity ID
        activity_id = self.get_latest_activity_id(activity_id, activity_type)

        if activity_id is None:
            return None  # No matching activity found

        activity_id = int(activity_id)

        query = """
            SELECT id, date, title, distance, duration, elevation_gain, file_id, activity_type
            FROM activities
            WHERE id = ?
        """
        activity = pd.read_sql(query, self.conn, params=(activity_id,))

        if activity.empty:
            return None  # No matching activity found

        activity = activity.iloc[0]  # Extract row

        # Format date
        formatted_date = self.format_date(activity["date"])

        # Convert duration to HH:MM:SS format
        duration = str(timedelta(seconds=int(activity["duration"])))

        # Check if track image exists
        field_id = activity["file_id"]
        track_path = os.path.join(self.file_path, f"{field_id}_track.png")
        track = track_path if os.path.exists(track_path) else None

        # Query media table for first available entry
        media_query = """
            SELECT media_type, file_path
            FROM media
            WHERE activity_id = ?
            LIMIT 1
        """
        media = pd.read_sql(media_query, self.conn, params=(activity_id,))

        media_info = None
        if not media.empty:
            media_info = {
                "media_type": media.iloc[0]["media_type"],
                "file_path": media.iloc[0]["file_path"],
            }

        # Query weather table for available weather data
        weather_query = """
            SELECT max_temp, min_temp, precipitation, max_wind_speed, weather_code
            FROM weather
            WHERE activity_id = ?
        """
        weather = pd.read_sql(weather_query, self.conn, params=(activity_id,))

        weather_info = None
        if not weather.empty:
            weather = weather.iloc[0]  # Get first row
            avg_temp = (
                (weather["max_temp"] + weather["min_temp"]) / 2
                if pd.notna(weather["max_temp"]) and pd.notna(weather["min_temp"])
                else None
            )

            weather_info = {
                "avg_temp": avg_temp,
                "precipitation": weather["precipitation"],
                "max_wind_speed": weather["max_wind_speed"],
                "weather_code": weather["weather_code"],
            }

        # Default extra data
        extra_data = {}

        current_type = map_activity_types(activity["activity_type"])

        # Query extra info based on activity type
        if current_type in [ViewMode.RUN, ViewMode.CYCLE, ViewMode.WALK]:
            table_name = "runs"
            if current_type == ViewMode.CYCLE:
                table_name = "cycling"
            elif current_type == ViewMode.WALK:
                table_name = "walking"

            query_extra = f"""
                SELECT avg_heart_rate, avg_pace
                FROM {table_name}
                WHERE activity_id = CAST(? AS INTEGER)
            """
            extra = pd.read_sql(query_extra, self.conn, params=(activity_id,))

            if not extra.empty:
                extra_data = extra.iloc[0].to_dict()

        return {
            "id": activity_id,
            "category": current_type,
            "date": formatted_date,
            "title": activity["title"],
            "distance": activity["distance"],  # Already in KM
            "duration": duration,
            "elevation_gain": activity["elevation_gain"],
            "track": track,
            "media": media_info,
            "weather": weather_info,
            "extra": extra_data,
        }
