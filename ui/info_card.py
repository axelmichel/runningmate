import pandas as pd
from PyQt6.QtCore import QLocale, Qt
from PyQt6.QtGui import QFont
from PyQt6.QtWidgets import QLabel, QVBoxLayout, QWidget

from database.database_handler import DatabaseHandler
from processing.system_settings import ViewMode, getAllowedTypes
from ui.themes import THEME
from utils.app_mode import is_dark_mode


class InfoCard(QWidget):
    def __init__(self, db_handler: DatabaseHandler, metric, parent=None):
        """
        Initializes an InfoCard for a specific metric.

        :param db_handler: DatabaseHandler, the database handler instance
        :param metric: str, the type of metric ("distance", "elevation", "duration")
        :param parent: QWidget, parent widget (optional)
        """
        super().__init__(parent)

        self.db = db_handler
        self.metric = metric.lower()  # Ensure lowercase for consistency

        # ✅ Set title based on metric
        metric_titles = {
            "distance": "Distance",
            "elevation": "Elevation Gain",
            "duration": "Duration",
            "activities": "Activities",
        }
        self.metric_title = metric_titles.get(self.metric, "")

        layout = QVBoxLayout()
        layout.setContentsMargins(10, 0, 10, 0)
        layout.setSpacing(1)

        value_color = THEME.ACCENT_COLOR if is_dark_mode() else THEME.MAIN_COLOR

        self.value_label = QLabel("0")
        self.value_label.setFont(QFont("Arial", 18, QFont.Weight.Bold))
        self.value_label.setAlignment(Qt.AlignmentFlag.AlignRight)
        self.value_label.setStyleSheet(f"color: {value_color};")

        self.description_label = QLabel(self.metric_title)
        self.description_label.setFont(QFont("Arial", 10))
        self.description_label.setAlignment(Qt.AlignmentFlag.AlignRight)

        layout.addWidget(self.value_label)
        layout.addWidget(self.description_label)
        layout.addStretch()

        self.setLayout(layout)

        self.update_info()

    def update_info(self, activity_type=None, filters=None):
        """
        Updates the info card with the selected metric.

        :param activity_type: str, filter by activity type (optional, default is None = all activities)
        """
        column_map = {
            "distance": "SUM(distance)",
            "elevation": "SUM(elevation_gain)",
            "duration": "SUM(duration)",
            "activities": "COUNT(*)",
        }
        column = column_map.get(self.metric)
        filter_params = None
        if not column:
            return

        query = f"SELECT {column} FROM activities"
        params = ()

        if activity_type is not None and activity_type != ViewMode.ALL:
            allowed_types = getAllowedTypes(activity_type)
            placeholders = ", ".join("?" * len(allowed_types))  # Generate placeholders
            query += f" WHERE activity_type IN ({placeholders})"
            params = tuple(allowed_types)
        if filters:
            query += " WHERE 1=1"
            query = self.db.add_filter_to_query(query, filters)
            filter_params = self.db.get_filter_params(filters)

        if filter_params:
            params += tuple(filter_params)
        df = pd.read_sql(query, self.db.conn, params=params)
        value = df.iloc[0, 0] if not df.empty else 0

        locale = QLocale.system()

        if self.metric == "duration":
            formatted_value = self.format_duration(value)
        elif self.metric == "distance":
            formatted_value = f"{locale.toString(value, 'f', 2)} km"
        elif self.metric == "elevation":
            formatted_value = f"{locale.toString(value, 'f', 0)} m"
        else:
            formatted_value = str(value)

        self.value_label.setText(formatted_value)

    @staticmethod
    def format_duration(seconds):
        """
        Converts seconds to hh:mm:ss format.

        :param seconds: int, duration in seconds
        :return: str, formatted duration
        """
        hours = seconds // 3600
        minutes = (seconds % 3600) // 60
        return f"{hours:02}:{minutes:02}"
