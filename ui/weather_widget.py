from concurrent.futures import ThreadPoolExecutor

import pandas as pd
from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtWidgets import (
    QFrame,
    QHBoxLayout,
    QLabel,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from processing.weather import WeatherService
from ui.icon_label import IconLabel
from utils.app_mode import is_dark_mode
from utils.resource_path import resource_path
from utils.translations import WMO_UNKNOWN, translate_weather_code, weather_code_icon

weather_executor = ThreadPoolExecutor(max_workers=2)


class WeatherWidget(QWidget):
    """
    A QWidget to display weather information
    """

    def __init__(self, db_handler: DatabaseHandler, activity_id: int, parent=None):
        """
        Initialize the activity widget with activity data.

        :param activity_id: int Id of the activity
        :param parent: QWidget, optional
            The parent widget.
        """
        super().__init__(parent)
        self.db = db_handler
        self.activity_id = activity_id
        self.init_ui()

    def _get_weather_info(self):
        """
        Attempts to load weather from DB, or asynchronously fetches it.
        Returns immediately with data if available, or None otherwise.
        """
        weather_query = """
            SELECT max_temp, min_temp, precipitation, max_wind_speed, weather_code
            FROM weather
            WHERE activity_id = ?
        """
        weather = pd.read_sql(weather_query, self.db.conn, params=(self.activity_id,))

        if not weather.empty:
            weather_data = weather.iloc[0]
            return self._format_weather(weather_data)
        else:
            weather_executor.submit(self._call_and_save_weather)

    def _call_and_save_weather(self):
        data = self._collect_weather_info_sync()
        if data:
            self._save_weather_info(data)
            QTimer.singleShot(0, lambda: self._on_weather_loaded(data))

    @staticmethod
    def _format_weather(weather_data: dict) -> dict:
        avg_temp = (
            (weather_data["max_temp"] + weather_data["min_temp"]) / 2
            if pd.notna(weather_data["max_temp"]) and pd.notna(weather_data["min_temp"])
            else None
        )
        return {
            "avg_temp": avg_temp,
            "precipitation": weather_data["precipitation"],
            "max_wind_speed": weather_data["max_wind_speed"],
            "weather_code": weather_data["weather_code"],
        }

    def _on_weather_loaded(self, weather_data: dict):
        formatted = self._format_weather(weather_data)

        def update_ui():
            self._render_weather_ui(formatted)

        QTimer.singleShot(0, update_ui)  # Schedules update_ui in the Qt main thread

    def _collect_weather_info_sync(self) -> dict | None:
        query = """
                 SELECT seg_latitude, seg_longitude, seg_time_start
                 FROM activity_details
                 WHERE activity_id = ? AND segment_id = 0
                """
        lat_lon = pd.read_sql(query, self.db.conn, params=(self.activity_id,))
        if not lat_lon.empty:
            lat = lat_lon.iloc[0]["seg_latitude"]
            lon = lat_lon.iloc[0]["seg_longitude"]
            date = lat_lon.iloc[0]["seg_time_start"].split(" ")[0]
            return WeatherService.get_weather(lat, lon, date)
        return None

    def _save_weather_info(self, weather_info):
        """Save weather information to the database."""
        self.db.conn.execute(
            """
                INSERT INTO weather (activity_id, max_temp, min_temp, precipitation, max_wind_speed, weather_code)
                VALUES (?, ?, ?, ?, ?, ?);
                """,
            (
                self.activity_id,
                weather_info["max_temp"],
                weather_info["min_temp"],
                weather_info["precipitation"],
                weather_info["max_wind_speed"],
                weather_info["weather_code"],
            ),
        )
        self.db.conn.commit()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 20)
        self.setLayout(layout)

        weather_info = self._get_weather_info()
        if weather_info is not None:
            self._render_weather_ui(weather_info)

    def _render_weather_ui(self, weather_info: dict):
        weather_frame = QFrame()
        weather_layout = QVBoxLayout(weather_frame)
        weather_layout.setContentsMargins(0, 0, 0, 0)

        weather_code = weather_info.get("weather_code", WMO_UNKNOWN)
        weather_icon = weather_code_icon(weather_code)

        if weather_icon is not None:
            icon_label = IconLabel(weather_icon, translate_weather_code(weather_code))
            weather_layout.addWidget(icon_label)
        else:
            weather_code_label = QLabel(translate_weather_code(weather_code))
            font = weather_code_label.font()
            font.setPointSize(14)
            weather_code_label.setFont(font)
            weather_layout.addWidget(weather_code_label)

        # Temperature & Wind Speed (horizontally split)
        temp_wind_layout = QHBoxLayout()
        avg_temp = weather_info.get("avg_temp", "--")
        wind_speed = weather_info.get("max_wind_speed", "--")
        icon_folder = "light" if is_dark_mode() else "dark"
        temp_icon = resource_path(f"icons/{icon_folder}/temp-cold-line.svg")
        wind_icon = resource_path(f"icons/{icon_folder}/windy-line.svg")
        temp_label = IconLabel(
            temp_icon,
            f"{avg_temp:.1f}°C" if isinstance(avg_temp, (int, float)) else "--",
            Qt.AlignmentFlag.AlignLeft,
            20,
        )
        wind_label = IconLabel(
            wind_icon,
            f"{wind_speed} km/h" if isinstance(wind_speed, (int, float)) else "--",
            Qt.AlignmentFlag.AlignRight,
            20,
        )

        temp_wind_layout.addWidget(temp_label)
        temp_wind_layout.addStretch(1)
        temp_wind_layout.addWidget(wind_label)
        weather_layout.addLayout(temp_wind_layout)

        self.layout().addWidget(weather_frame)
