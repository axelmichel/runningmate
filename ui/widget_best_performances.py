import numpy as np
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont
from PyQt6.QtWidgets import QFrame, QHBoxLayout, QLabel, QVBoxLayout, QWidget

from ui.themes import THEME
from utils.app_mode import is_dark_mode


class BestPerformanceWidget(QWidget):
    """
    A QWidget to display the best performance data in a structured format.

    Each row contains:
    - Left-aligned distance (e.g., "10K")
    - Right-aligned pace (formatted as MM:SS min/km)
    - A fine separator line (except after the last row)
    """

    def __init__(self, best_performance_data: dict, parent=None):
        """
        Initializes the BestPerformanceWidget.

        :param best_performance_data: dict
            A dictionary containing the best performance details.
            Expected format:
            {
                "1K": {"seg_time_start": "2025-02-23 13:54:33+00:00",
                       "seg_time_end": "2025-02-23 14:45:19+00:00",
                       "seg_avg_pace": np.float64(9.16257129188063)},
                "5K": {"seg_time_start": "2025-02-23 13:54:33+00:00",
                       "seg_time_end": "2025-02-23 14:45:19+00:00",
                       "seg_avg_pace": np.float64(9.16257129188063)}
            }
        :param parent: QWidget, optional
            The parent widget.
        """
        super().__init__(parent)
        self.best_performance_data = best_performance_data
        self.init_ui()

    def init_ui(self):
        """Set up the layout and populate the UI with the best performance data."""
        if not self.best_performance_data:
            return

        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 20)
        value_color = THEME.ACCENT_COLOR if is_dark_mode() else THEME.MAIN_COLOR

        # Sort dictionary items by numeric distance (e.g., "1K" → 1, "5K" → 5)
        sorted_items = sorted(
            self.best_performance_data.items(),
            key=lambda x: int(x[0][:-1]),  # Extract numeric part from "1K", "5K"
        )

        for index, (distance_key, data) in enumerate(sorted_items):
            # Extract and format pace
            formatted_pace = self.format_pace(data["seg_avg_pace"])

            # Distance Label (Left-aligned)
            distance_label = QLabel(distance_key)
            distance_label.setFont(QFont("Arial", 14))
            distance_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

            # Pace Label (Right-aligned)
            pace_label = QLabel(formatted_pace)
            pace_label.setFont(QFont("Arial", 14, QFont.Weight.Bold))
            pace_label.setStyleSheet(f"color: {value_color};")
            pace_label.setAlignment(Qt.AlignmentFlag.AlignRight)

            # Row layout with distance (left) and pace (right)
            row_layout = QHBoxLayout()
            row_layout.setContentsMargins(0, 0, 0, 0)
            row_layout.addWidget(distance_label)
            row_layout.addStretch()  # Pushes pace to the right
            row_layout.addWidget(pace_label)

            row_widget = QWidget()
            row_widget.setLayout(row_layout)
            layout.addWidget(row_widget)

            # Add a fine separator line (except after the last row)
            if index < len(sorted_items) - 1:
                separator = QFrame()
                separator.setFrameShape(QFrame.Shape.HLine)
                separator.setFrameShadow(QFrame.Shadow.Sunken)
                separator.setStyleSheet("color: #333333;")  # Light gray fine line
                layout.addWidget(separator)

        self.setLayout(layout)

    @staticmethod
    def format_pace(pace: float) -> str:
        """
        Converts pace from float (min/km) to MM:SS format.

        :param pace: float
            The pace value in min/km.
        :return: str
            Formatted pace as MM:SS.
        """
        if isinstance(pace, np.float64):  # Convert np.float64 to standard float
            pace = float(pace)

        minutes = int(pace)
        seconds = int((pace - minutes) * 60)
        return f"{minutes:02d}:{seconds:02d}"
