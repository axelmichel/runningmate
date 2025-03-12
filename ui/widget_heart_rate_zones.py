from datetime import datetime, timedelta
from typing import Dict

from PyQt6.QtGui import QFont
from PyQt6.QtWidgets import QLabel, QVBoxLayout, QWidget

from database.database_handler import DatabaseHandler


class HeartRateZoneWidget(QWidget):
    def __init__(self, db_handler: DatabaseHandler, user_id: int, activity_id: int):
        """
        Initializes the HeartRateZoneWidget with database path and user ID.

        :param db_handler: DatabaseHandler instance.
        :param user_id: ID of the user to fetch heart rate zones.
        :param activity_id: ID of the activity to fetch heart rate data.
        """
        super().__init__()
        self.user_id = None
        self.db = db_handler
        self.user_id = user_id
        self.activity_id = activity_id
        self.zones = self._fetch_user_zones()
        self.init_ui()

    def init_ui(self):
        """
        Initializes the PyQt6 widget UI.
        """
        layout = QVBoxLayout()
        time_per_zone = self.calculate_time_per_zone()
        max_width = 40  # Fixed width for visualization

        for zone, percentage in time_per_zone.items():
            bar_length = int((percentage / 100) * max_width)
            bar = "■" * bar_length + "-" * (max_width - bar_length)
            label = QLabel(f"{zone.capitalize()} {bar} {int(percentage)}%")
            label.setFont(QFont("Courier", 10))
            layout.addWidget(label)

        self.setLayout(layout)

    def calculate_time_per_zone(self) -> Dict[str, float]:
        """
        Calculates the total time spent in each heart rate zone as a percentage.
        :return: Dictionary mapping each zone to total percentage of time spent.
        """
        self.db.cursor.execute("SELECT seg_avg_heart_rate, seg_time_start, seg_time_end FROM activity_details WHERE activity_id = ?", (self.activity_id,))

        zone_times = {f'zone_{i}': timedelta() for i in range(1, 6)}
        total_time = timedelta()

        for heart_rate, start_time, end_time in self.db.cursor.fetchall():
            start_dt = datetime.strptime(start_time, "%Y-%m-%d %H:%M:%S")
            end_dt = datetime.strptime(end_time, "%Y-%m-%d %H:%M:%S")
            duration = end_dt - start_dt

            zone = self._get_zone(heart_rate)
            zone_times[zone] += duration
            total_time += duration

        return {zone: (time.total_seconds() / total_time.total_seconds()) * 100 if total_time.total_seconds() > 0 else 0 for zone, time in zone_times.items()}


    def _fetch_user_zones(self) -> Dict[str, int]:
        """
        Fetches the heart rate zones for the user from the database.
        :return: Dictionary with zone thresholds.
        """
        self.db.cursor.execute("SELECT zone1, zone2, zone3, zone4, zone5 FROM users WHERE id = ?", (self.user_id,))
        row = self.db.cursor.fetchone()

        if row is None:
            raise ValueError("User ID not found in the database.")

        return row

    def _get_zone(self, heart_rate: int) -> str:
        """
        Determines which heart rate zone a given heart rate belongs to.
        :param heart_rate: Heart rate value.
        :return: The corresponding zone name.
        """
        for i in range(5, 0, -1):
            if heart_rate >= self.zones[f'zone{i}']:
                return f'zone{i}'
        return 'zone1'