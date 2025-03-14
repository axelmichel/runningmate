from datetime import datetime, timedelta
from typing import Dict

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QApplication,
    QHBoxLayout,
    QLabel,
    QProgressBar,
    QSizePolicy,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from ui.themes import THEME
from utils.translations import _


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
        Initializes the PyQt6 widget UI with properly scaled and colored progress bars.
        """
        layout = QVBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)

        title_label = QLabel(_("Heart Rate Zones"))
        title_label.setStyleSheet(
            """
            font-size: 13px;
            font-weight: bold;
            margin-bottom: 10px;
        """
        )
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        layout.addWidget(title_label)

        time_per_zone = self.calculate_time_per_zone()

        # Get the available screen width
        screen_width = int(QApplication.primaryScreen().availableGeometry().width())
        dialog_max_width = min(int(screen_width * 0.8), 600)
        self.setMaximumWidth(dialog_max_width)

        label_width = 150  # Fixed width for zone label
        percentage_width = 40  # Fixed width for percentage label
        bar_min_width = 200

        # Define different colors for each zone
        zone_colors = {
            "zone1": THEME.MAIN_COLOR_DARK,
            "zone2": THEME.MAIN_COLOR,
            "zone3": THEME.MAIN_COLOR_LIGHT,
            "zone4": THEME.ACCENT_COLOR,
            "zone5": THEME.ACCENT_COLOR_LIGHT,
        }

        for zone, percentage in time_per_zone.items():
            min_hr, max_hr = self._get_zone_range(zone)  # Get HR range for the zone

            row_layout = QHBoxLayout()
            row_layout.setContentsMargins(5, 0, 0, 0)
            row_layout.setAlignment(Qt.AlignmentFlag.AlignLeft)

            zone_label = QLabel(
                f"{_(zone.capitalize())} ({min_hr}-{max_hr} {_("BPM")}) "
            )
            zone_label.setFixedWidth(label_width)
            zone_label.setAlignment(
                Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
            )

            # Progress Bar (Properly Scaled & Colored)
            bar = QProgressBar()
            bar.setRange(0, 100)
            bar.setValue(int(percentage))
            bar.setTextVisible(False)  # Hide the percentage inside the progress bar
            bar.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
            bar.setMinimumWidth(bar_min_width)
            bar.setAlignment(Qt.AlignmentFlag.AlignVCenter)

            # Apply color based on zone
            bar_color = zone_colors.get(zone, "#808080")  # Default to gray if not found
            bar.setStyleSheet(
                f"""
                QProgressBar {{
                    border: 1px solid #000;
                    border-radius: 3px;
                    min-height: 10px;
                    max-height: 12px;
                    background: {THEME.SYSTEM_BUTTON};
                }}
                QProgressBar::chunk {{
                    background: {bar_color};
                }}
            """
            )

            # Percentage Label (Fixed Width)
            percentage_label = QLabel(f"{int(percentage)}%")
            percentage_label.setFixedWidth(percentage_width)
            percentage_label.setAlignment(
                Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter
            )

            # Add widgets to the row
            row_layout.addWidget(zone_label)
            row_layout.addWidget(bar, 1)
            row_layout.addWidget(percentage_label)

            # Add row to main layout
            layout.addLayout(row_layout)

        self.setLayout(layout)

    def calculate_time_per_zone(self) -> Dict[str, float]:
        """
        Calculates the total time spent in each heart rate zone as a percentage.
        :return: Dictionary mapping each zone to total percentage of time spent.
        """
        self.db.cursor.execute(
            "SELECT seg_avg_heart_rate, seg_time_start, seg_time_end FROM activity_details WHERE activity_id = ?",
            (self.activity_id,),
        )

        zone_times = {f"zone{i}": timedelta() for i in range(1, 6)}
        total_time = timedelta()

        for heart_rate, start_time, end_time in self.db.cursor.fetchall():
            start_time = start_time.split("+")[0]  # Remove timezone
            end_time = end_time.split("+")[0]  # Remove
            start_dt = datetime.strptime(start_time, "%Y-%m-%d %H:%M:%S")
            end_dt = datetime.strptime(end_time, "%Y-%m-%d %H:%M:%S")
            duration = end_dt - start_dt
            zone = self._get_zone(heart_rate)
            zone_times[zone] += duration
            total_time += duration

        return {
            zone: (
                (time.total_seconds() / total_time.total_seconds()) * 100
                if total_time.total_seconds() > 0
                else 0
            )
            for zone, time in zone_times.items()
        }

    def _fetch_user_zones(self) -> Dict[str, int]:
        """
        Fetches the heart rate zones for the user from the database.
        :return: Dictionary with zone thresholds.
        """
        self.db.cursor.execute(
            "SELECT zone1, zone2, zone3, zone4, zone5 FROM users WHERE id = ?",
            (self.user_id,),
        )
        row = self.db.cursor.fetchone()

        if row is None:
            raise ValueError("User ID not found in the database.")

        return {f"zone{i+1}": row[i] if row[i] is not None else 0 for i in range(5)}

    def _get_zone(self, heart_rate: int) -> str:
        """
        Determines which heart rate zone a given heart rate belongs to.
        :param heart_rate: Heart rate value.
        :return: The corresponding zone name.
        """
        for i in range(5, 0, -1):
            if heart_rate >= self.zones[f"zone{i}"]:
                return f"zone{i}"
        return "zone1"

    def _get_zone_range(self, zone: str) -> tuple:
        """
        Returns the min and max heart rate values for a given zone.

        :param zone: The zone name (e.g., "zone1").
        :return: Tuple (min_hr, max_hr).
        """
        zone_number = int(zone[-1])  # Extract the zone number (1-5)

        max_hr = self.zones[f"zone{zone_number}"]  # Get the max HR for this zone

        if zone_number == 1:
            # Calculate Zone1 min as 50% of HRmax (Zone 5 max HR)
            min_hr = int(self.zones["zone5"] * 0.5)
        else:
            # Min HR is the previous zone's max +1
            min_hr = self.zones[f"zone{zone_number - 1}"] + 1

        return min_hr, max_hr
