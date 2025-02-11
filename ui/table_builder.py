from PyQt6.QtWidgets import QTableWidget, QTableWidgetItem, QHeaderView, QPushButton, QHBoxLayout, QVBoxLayout, QWidget
from PyQt6.QtCore import Qt
from translations import _  # Import translation function


class NumericTableWidgetItem(QTableWidgetItem):
    """Custom TableWidgetItem that ensures proper numeric sorting."""

    def __init__(self, value):
        super().__init__(str(value))
        try:
            self.numeric_value = float(value)
        except ValueError:
            self.numeric_value = value

    def __lt__(self, other):
        if isinstance(other, NumericTableWidgetItem):
            try:
                return self.numeric_value < other.numeric_value
            except TypeError:
                return str(self.numeric_value) < str(other.numeric_value)
        return super().__lt__(other)


HEADERS = {
    "all": ["activity_type", "date", "start_time", "title", "duration", "distance"],
    "runs": ["date", "start_time", "distance", "total_time", "elevation_gain", "avg_speed",
             "avg_steps", "total_steps", "avg_power", "avg_heart_rate", "avg_pace",
             "fastest_pace", "slowest_pace", "pause"],
    "walks": ["date", "start_time", "distance", "total_time", "elevation_gain", "avg_speed",
              "total_steps", "avg_power", "avg_heart_rate", "avg_pace"],
    "cycling": ["date", "start_time", "distance", "total_time", "elevation_gain", "avg_speed",
                "avg_power", "avg_heart_rate", "avg_pace", "fastest_pace", "slowest_pace", "pause"]
}


class TableBuilder:
    COLUMN_ALIGNMENTS = {
        "distance": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "avg_speed": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "date": Qt.AlignmentFlag.AlignCenter | Qt.AlignmentFlag.AlignVCenter
    }

    @staticmethod
    def setup_table(table_widget: QTableWidget, activity_type: str, data: list):
        if not data:
            return  # No data to display

        headers = HEADERS.get(activity_type, [])  # Get defined headers in order
        translated_headers = [_((header)) for header in headers]  # Translate headers

        table_widget.setRowCount(len(data))
        table_widget.setColumnCount(len(headers))
        table_widget.setHorizontalHeaderLabels(translated_headers)
        table_widget.horizontalHeader().setSectionResizeMode(QHeaderView.ResizeMode.Stretch)

        for row_index, row_data in enumerate(data):
            for col_index, key in enumerate(headers):
                value = row_data.get(key, "")  # Get value from dict, default to empty
                item = NumericTableWidgetItem(value)

                # Apply alignment based on COLUMN_ALIGNMENTS dictionary
                if key in TableBuilder.COLUMN_ALIGNMENTS:
                    item.setTextAlignment(TableBuilder.COLUMN_ALIGNMENTS[key])
                else:
                    item.setTextAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)

                table_widget.setItem(row_index, col_index, item)
