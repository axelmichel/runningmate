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

    def __init__(self, activity_info: dict, display_title: bool = True, parent=None):
        """
        Initialize the activity widget with activity data.

        :param activity_info: dict
            A dictionary containing activity details.
        :param display_title: bool, optional
        :param parent: QWidget, optional
            The parent widget.
        """
        super().__init__(parent)
        self.display_title = display_title
        self.activity_info = activity_info
        self.init_ui()

    def init_ui(self):
        """Set up the layout and populate the UI with activity data."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 20)
        layout.setSpacing(1)
        value_color = THEME.ACCENT_COLOR if is_dark_mode() else THEME.MAIN_COLOR
        locale = QLocale.system()

        # Line 1: Date
        if self.display_title:
            title_layout = QVBoxLayout()
            date_label = QLabel(self.activity_info.get("date", "Unknown Date"))
            date_label.setFont(QFont("Arial", 16, QFont.Weight.Bold))
            title_layout.addWidget(date_label)

            title_label = QLabel(self.activity_info.get("title", "Untitled Activity"))
            title_label.setFont(QFont("Arial", 12))
            title_layout.addWidget(title_label)
            title_layout.setSpacing(5)
            layout.addLayout(title_layout)

        # Line 2: Duration (Left) and Pace (Right)
        duration_pace_layout = QVBoxLayout()
        from_top = self.display_title and 10 or 0
        duration_pace_layout.setContentsMargins(0, from_top, 0, 10)
        duration_pace_container = QHBoxLayout()  # Holds values
        duration_pace_labels = QHBoxLayout()  # Holds text labels

        duration_label = QLabel(f"{self.activity_info.get('duration', '00:00:00')}")
        duration_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        duration_label.setFont(QFont("Arial", 20, QFont.Weight.Bold))
        duration_label.setStyleSheet(f"color: {value_color};")

        duration_text_label = QLabel("Duration")
        duration_text_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        duration_text_label.setFont(QFont("Arial", 10))

        avg_pace = self.activity_info.get("extra", {}).get("avg_pace", "--")
        avg_pace_label = QLabel(f"{avg_pace}")
        avg_pace_label.setAlignment(Qt.AlignmentFlag.AlignRight)
        avg_pace_label.setFont(QFont("Arial", 20, QFont.Weight.Bold))
        avg_pace_label.setStyleSheet(f"color: {value_color};")

        pace_text_label = QLabel("Pace")
        pace_text_label.setAlignment(Qt.AlignmentFlag.AlignRight)
        pace_text_label.setFont(QFont("Arial", 10))

        # Add Labels and Texts to Layouts
        duration_pace_container.addWidget(duration_label)
        duration_pace_container.addWidget(avg_pace_label)

        duration_pace_labels.addWidget(duration_text_label)
        duration_pace_labels.addWidget(pace_text_label)

        # Stack both layouts (values + text labels)
        duration_pace_layout.addLayout(duration_pace_container)
        duration_pace_layout.addLayout(duration_pace_labels)

        # Add to Main Layout
        layout.addLayout(duration_pace_layout)

        # Line 3: Track Image (if available)
        track_path = self.activity_info.get("track", None)
        if os.path.exists(track_path or ""):
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
        metrics_layout.setContentsMargins(0, 10, 0, 0)

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

        self.setLayout(layout)
