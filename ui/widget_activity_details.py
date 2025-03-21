from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QHeaderView,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from processing.system_settings import ViewMode
from utils.translations import _


class ActivityDetailsWidget(QWidget):
    def __init__(self, db_handler, activity_id, activity_type):
        super().__init__()
        self.db_handler = db_handler
        self.activity_id = activity_id
        self.activity_type = activity_type
        self.default_distance = 5 if self.activity_type == ViewMode.CYCLE else 1

        self.layout = QVBoxLayout()
        self.table = QTableWidget()
        self.table.setAlternatingRowColors(True)
        self.table.setShowGrid(True)
        self.table.setGridStyle(Qt.PenStyle.SolidLine)
        self.layout.addWidget(self.table)
        self.setLayout(self.layout)

        self.table.cellChanged.connect(
            self.update_database
        )  # Connect signal for updating DB
        self.load_data()

    def load_data(self):
        """Loads segment data from the database and populates the table."""
        query = """
        SELECT segment_id, seg_avg_heart_rate, seg_avg_power, seg_avg_speed, seg_avg_pace, seg_distance, seg_elevation_gain,
               seg_avg_steps
        FROM activity_details WHERE activity_id = ?
        """

        cursor = self.db_handler.conn.cursor()
        cursor.execute(query, (self.activity_id,))
        rows = cursor.fetchall()
        cursor.close()

        columns = [
            _("AVG HR"),
            _("AVG Power"),
            _("AVG Speed (km/h)"),
            _("AVG Pace (min/km)"),
            _("Distance"),
            _("Elevation Gain"),
        ]
        if self.activity_type != ViewMode.CYCLE:
            columns.append("Avg Steps")

        self.table.setColumnCount(len(columns))
        self.table.setHorizontalHeaderLabels(columns)
        self.table.setRowCount(len(rows))
        self.table.horizontalHeader().setSectionResizeMode(
            QHeaderView.ResizeMode.ResizeToContents
        )

        self.rowid_map = {}  # Map row index to segment_id in DB
        for row_idx, row in enumerate(rows):
            (
                segment_id,
                seg_avg_heart_rate,
                seg_avg_power,
                seg_avg_speed,
                seg_avg_pace,
                seg_distance,
                seg_elevation_gain,
                seg_avg_steps,
            ) = row

            displayed_distance = (
                self.default_distance
                if seg_distance >= self.default_distance
                else round(seg_distance, 2)
            )
            formatted_speed = round(seg_avg_speed * 3.6, 2)  # Convert m/s to km/h
            pace_minutes = int((1000 / seg_avg_speed) // 60)  # Extract whole minutes
            pace_seconds = int((1000 / seg_avg_speed) % 60)  # Extract remaining seconds
            formatted_pace = f"{pace_minutes}:{pace_seconds:02d}"  # Format as mm:ss/km

            row_data = [
                int(seg_avg_heart_rate),
                int(seg_avg_power),
                formatted_speed,
                formatted_pace,
                displayed_distance,
                int(seg_elevation_gain * 2),  # Elevation gain doubled
            ]

            if self.activity_type != ViewMode.CYCLE:
                row_data.append(int(seg_avg_steps * 2))  # Steps doubled

            self.rowid_map[row_idx] = segment_id  # Store segment_id for updates

            for col_idx, value in enumerate(row_data):
                item = QTableWidgetItem(str(value))
                item.setFlags(
                    item.flags() | Qt.ItemFlag.ItemIsEditable
                )  # Enable item editing
                item.setTextAlignment(
                    Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter
                )
                self.table.setItem(row_idx, col_idx, item)

        self.db_handler.conn.commit()

    def update_database(self, row, column):
        """Updates the database when a cell is edited."""
        segment_id = self.rowid_map.get(row)
        if segment_id is None:
            return

        column_names = [
            "seg_avg_heart_rate",
            "seg_avg_power",
            "seg_avg_speed",
            "seg_avg_pace",
            "seg_distance",
            "seg_elevation_gain",
        ]
        if self.activity_type != ViewMode.CYCLE:
            column_names.append("seg_avg_steps")

        new_value = self.table.item(row, column).text()
        column_name = column_names[column]

        # Ensure proper data types for storage
        if column in [0, 1]:  # HR and Power (Integers)
            new_value = int(float(new_value))
        elif column == 2:  # Speed (Convert back to m/s)
            new_value = float(new_value) / 3.6
        elif column == 3:  # Pace (Convert mm:ss back to seconds per km, then to m/s)
            minutes, seconds = map(int, new_value.split(":"))
            total_seconds = (minutes * 60) + seconds
            new_value = 1000 / total_seconds
        elif column == 4:  # Distance (Keep as is)
            new_value = float(new_value)
        elif column == 5:  # Elevation Gain (Doubled)
            new_value = int(float(new_value) / 2)
        elif column == 6 and self.activity_type != ViewMode.CYCLE:  # Steps (Doubled)
            new_value = int(float(new_value) / 2)

        query = f"UPDATE activity_details SET {column_name} = ? WHERE segment_id = ? AND activity_id = ?"
        cursor = self.db_handler.conn.cursor()
        cursor.execute(query, (new_value, segment_id, self.activity_id))
        self.db_handler.conn.commit()
