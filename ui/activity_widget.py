import os

from PyQt6.QtCore import QLocale, Qt
from PyQt6.QtGui import QFont, QPixmap
from PyQt6.QtWidgets import (
    QFrame,
    QGridLayout,
    QHBoxLayout,
    QLabel,
    QVBoxLayout,
    QWidget,
)

from ui.themes import THEME
from utils.app_mode import is_dark_mode
from utils.translations import WMO_UNKNOWN, translate_weather_code


class ActivityWidget(QWidget):
    """
    A QWidget to display activity information in a structured layout.
    """

    def __init__(self, activity_info: dict, parent=None):
        """
        Initialize the activity widget with activity data.

        :param activity_info: dict
            A dictionary containing activity details.
        :param parent: QWidget, optional
            The parent widget.
        """
        super().__init__(parent)

        self.activity_info = activity_info
        self.init_ui()

    def init_ui(self):
        """Set up the layout and populate the UI with activity data."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(10, 0, 10, 0)
        value_color = THEME.ACCENT_COLOR if is_dark_mode() else THEME.MAIN_COLOR
        locale = QLocale.system()

        # Line 1: Date
        date_label = QLabel(self.activity_info.get("date", "Unknown Date"))
        date_label.setFont(QFont("Arial", 16, QFont.Weight.Bold))
        layout.addWidget(date_label)

        title_label = QLabel(self.activity_info.get("title", "Untitled Activity"))
        title_label.setFont(QFont("Arial", 12))
        layout.addWidget(title_label)

        duration_pace_layout = QHBoxLayout()
        duration_label = QLabel(f"{self.activity_info.get('duration', '00:00:00')}")
        duration_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        duration_label.setFont(QFont("Arial", 20, QFont.Weight.Bold))
        duration_label.setStyleSheet(f"color: {value_color};")

        avg_pace = self.activity_info.get("extra", {}).get("avg_pace", None)
        avg_pace_label = QLabel(f"{avg_pace}" if avg_pace else "--")
        avg_pace_label.setAlignment(Qt.AlignmentFlag.AlignRight)
        avg_pace_label.setFont(QFont("Arial", 20, QFont.Weight.Bold))
        avg_pace_label.setStyleSheet(f"color: {value_color};")

        duration_pace_layout.addWidget(duration_label)
        duration_pace_layout.addWidget(avg_pace_label)
        layout.addLayout(duration_pace_layout)

        # Line 3: Track Image (if available)
        track_path = self.activity_info.get("track", None)
        if track_path and os.path.exists(track_path):
            track_label = QLabel()
            pixmap = QPixmap(track_path)
            pixmap = pixmap.scaled(
                400,
                200,
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation,
            )
            track_label.setPixmap(pixmap)
            layout.addWidget(track_label)

        # Line 5-6: Distance & Elevation Gain (Grid Layout)
        metrics_layout = QGridLayout()

        distance_value = float(self.activity_info.get("distance", 0))
        elevation_value = float(self.activity_info.get("elevation_gain", 0))

        # Distance block
        distance_label = QLabel(f"{locale.toString(distance_value, 'f', 2)} km")
        distance_label.setFont(QFont("Arial", 18, QFont.Weight.Bold))
        distance_label.setStyleSheet(f"color: {value_color};")
        distance_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

        distance_text_label = QLabel("Distance")
        distance_text_label.setFont(QFont("Arial", 10))
        distance_text_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

        metrics_layout.addWidget(distance_label, 0, 0)
        metrics_layout.addWidget(distance_text_label, 1, 0)

        # Elevation Gain block
        elevation_label = QLabel(f"{locale.toString(elevation_value, 'f', 0)} m")
        elevation_label.setFont(QFont("Arial", 18, QFont.Weight.Bold))
        elevation_label.setStyleSheet(f"color: {value_color};")
        elevation_label.setAlignment(Qt.AlignmentFlag.AlignRight)

        elevation_text_label = QLabel("Elevation")
        elevation_text_label.setFont(QFont("Arial", 10))
        elevation_text_label.setAlignment(Qt.AlignmentFlag.AlignRight)

        metrics_layout.addWidget(elevation_label, 0, 1)
        metrics_layout.addWidget(elevation_text_label, 1, 1)

        layout.addLayout(metrics_layout)

        # Weather Info (if available)
        weather_info = self.activity_info.get("weather", None)
        if weather_info:
            weather_frame = QFrame()
            weather_layout = QVBoxLayout(weather_frame)
            weather_layout.setContentsMargins(0, 20, 0, 0)

            # Weather Code
            weather_code = weather_info.get("weather_code", WMO_UNKNOWN)
            weather_code_label = QLabel(translate_weather_code(weather_code))
            weather_code_label.setFont(QFont("Arial", 14))
            weather_layout.addWidget(weather_code_label)

            # Temperature & Wind Speed (horizontally split)
            temp_wind_layout = QHBoxLayout()
            avg_temp = weather_info.get("avg_temp", "--")
            wind_speed = weather_info.get("max_wind_speed", "--")

            temp_label = QLabel(
                f"{avg_temp:.1f}°C" if isinstance(avg_temp, (int, float)) else "--"
            )
            wind_label = QLabel(
                f"{wind_speed} km/h" if isinstance(wind_speed, (int, float)) else "--"
            )

            temp_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
            wind_label.setAlignment(Qt.AlignmentFlag.AlignRight)

            temp_wind_layout.addWidget(temp_label)
            temp_wind_layout.addWidget(wind_label)

            weather_layout.addLayout(temp_wind_layout)
            layout.addWidget(weather_frame)

        self.setLayout(layout)
