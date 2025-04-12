from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import QFrame, QHBoxLayout, QLabel, QVBoxLayout, QWidget

from database.database_handler import DatabaseHandler
from ui.themes import THEME
from utils.app_mode import is_dark_mode
from utils.translations import _


class RunDetailsWidget(QWidget):

    def __init__(self, db_handler: DatabaseHandler, activity_id: int, parent=None):
        super().__init__(parent)
        self.db = db_handler
        self.activity_id = activity_id
        self.activity = self._get_activity_data()
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 20)
        value_color = THEME.ACCENT_COLOR if is_dark_mode() else THEME.MAIN_COLOR

        # loop through the activity data and display the data
        for index, (key, value) in enumerate(self.activity.items()):

            key_label = QLabel(_(key.title()))
            font = key_label.font()
            font.setPointSize(14)
            key_label.setFont(font)
            key_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

            value_label = QLabel(str(value))
            font = value_label.font()
            font.setPointSize(14)
            value_label.setFont(font)
            value_label.setStyleSheet(f"color: {value_color};")
            value_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

            row_layout = QHBoxLayout()
            row_layout.setContentsMargins(0, 0, 0, 0)
            row_layout.addWidget(key_label)
            row_layout.addStretch()
            row_layout.addWidget(value_label)

            row_widget = QWidget()
            row_widget.setLayout(row_layout)
            layout.addWidget(row_widget)

            # Add a fine separator line (except after the last row)
            if index < len(self.activity.items()) - 1:
                separator = QFrame()
                separator.setFrameShape(QFrame.Shape.HLine)
                separator.setFrameShadow(QFrame.Shadow.Sunken)
                separator.setStyleSheet(f"color: {THEME.SYSTEM_BUTTON}")
                layout.addWidget(separator)

        self.setLayout(layout)

    def _get_activity_data(self) -> dict | None:
        """
        Get the activity data from the database.

        Returns:
            dict | None: Dictionary with activity details or None if not found.
        """
        activity_data = self.db.fetch_run_by_activity_id(self.activity_id)
        if activity_data is None:
            return None
        data = {
            "calories": activity_data["calories"],
            "avg_power": activity_data["avg_power"],
            "avg_steps": activity_data["avg_steps"],
            "total_steps": activity_data["total_steps"],
            "slowest_pace": activity_data["slowest_pace"],
            "avg_pace": activity_data["avg_pace"],
            "fastest_pace": activity_data["fastest_pace"],
            "pause": activity_data["pause"],
        }

        shoe_name = activity_data.get("shoe_name")
        if shoe_name and str(shoe_name).strip():
            data["shoe"] = shoe_name

        return data
