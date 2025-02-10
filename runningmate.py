import sys
import os
from PyQt6.QtWidgets import (
    QApplication, QWidget, QVBoxLayout, QPushButton, QFileDialog, QLabel,
    QTableWidget, QTableWidgetItem, QComboBox, QMessageBox, QHeaderView, QSplashScreen
)
from PyQt6.QtGui import QPixmap, QIcon
from PyQt6.QtCore import Qt, QTimer

from processing.compute_statistics import compute_run_db_data
from processing.data_processing import convert_to_utm, calculate_distance, calculate_pace, detect_pauses, calculate_steps
from processing.parse_tcx import parse_tcx
from processing.visualization import plot_track, plot_elevation, plot_activity_map

from database.database_handler import initialize_database, insert_run, get_years, get_months, get_runs, get_run_by_id
from ui.window_run_details import RunDetailsWindow

# Directories
IMG_DIR = os.path.expanduser("~/RunningData/images")

if not os.path.exists(IMG_DIR):
    os.makedirs(IMG_DIR)


class NumericTableWidgetItem(QTableWidgetItem):
    """Custom TableWidgetItem that ensures proper numeric sorting."""
    def __init__(self, value):
        super().__init__(str(value))  # Store as text
        try:
            self.numeric_value = float(value)  # Convert to float for sorting
        except ValueError:
            self.numeric_value = value  # Keep original if not a number

    def __lt__(self, other):
        if isinstance(other, NumericTableWidgetItem):
            try:
                return self.numeric_value < other.numeric_value
            except TypeError:
                return str(self.numeric_value) < str(other.numeric_value)
        return super().__lt__(other)


class RunningDataApp(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowIcon(QIcon("icon.png"))  # Set custom icon
        self.last_sorted_column = None  # Track last clicked column
        self.sort_order = Qt.SortOrder.AscendingOrder  # Default sort order
        self.initUI()

    def initUI(self):
        layout = QVBoxLayout()

        self.uploadButton = QPushButton("Upload and Process TCX File")
        self.uploadButton.clicked.connect(self.upload_tcx_file)
        layout.addWidget(self.uploadButton)

        self.yearComboBox = QComboBox()
        self.monthComboBox = QComboBox()
        self.yearComboBox.currentIndexChanged.connect(self.load_months)
        self.monthComboBox.currentIndexChanged.connect(self.load_runs)

        layout.addWidget(QLabel("Select Year:"))
        layout.addWidget(self.yearComboBox)
        layout.addWidget(QLabel("Select Month:"))
        layout.addWidget(self.monthComboBox)

        self.tableWidget = QTableWidget()
        self.tableWidget.setSortingEnabled(True)  # Enable sorting
        self.tableWidget.horizontalHeader().setSectionResizeMode(QHeaderView.ResizeMode.Stretch)

        layout.addWidget(self.tableWidget)

        self.setLayout(layout)
        self.setWindowTitle("Running Mate")
        self.load_years()

    def center_window(self):
        """Centers the window on the primary screen."""
        screen = QApplication.primaryScreen().geometry()
        window_rect = self.geometry()

        x = (screen.width() - window_rect.width()) // 2
        y = (screen.height() - window_rect.height()) // 2

        self.move(x, y)

    def load_years(self):
        years = get_years()
        self.yearComboBox.addItems(years)
        if years:
            self.yearComboBox.setCurrentText(years[0])
            self.load_months()

    def load_months(self):
        selected_year = self.yearComboBox.currentText()
        if selected_year:
            months = get_months(selected_year)
            self.monthComboBox.clear()
            self.monthComboBox.addItems(months)
            if months:
                self.monthComboBox.setCurrentText(months[0])
                self.load_runs()

    def load_runs(self):
        """Loads runs from the database and displays them in the table."""
        selected_year = self.yearComboBox.currentText()
        selected_month = self.monthComboBox.currentText()

        if not selected_year or not selected_month:
            return

        runs = get_runs(selected_year, selected_month)

        headers = [
            "Date", "Start Time", "Distance (km)", "Total Time", "Elevation Gain (m)",
            "Avg Speed (km/h)", "Avg Steps (SPM)", "Total Steps", "Avg Power (Watts)",
            "Avg Heart Rate (BPM)", "Avg Pace", "Fastest Pace", "Slowest Pace", "Pause", "Activity Type"
        ]

        self.tableWidget.setRowCount(len(runs))
        self.tableWidget.setColumnCount(len(headers))
        self.tableWidget.setHorizontalHeaderLabels(headers)

        # Disconnect previous connections before reconnecting
        try:
            self.tableWidget.cellClicked.disconnect(self.handle_cell_clicked)
        except TypeError:
            pass
        self.tableWidget.cellClicked.connect(self.handle_cell_clicked)

        self.run_id_mapping = {}

        right_align_columns = {1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13}  # Indices of numeric columns

        for i, run in enumerate(runs):
            run_id = run[0]
            run_date = run[1]
            run_start_time = run[4]

            # Store (date, start_time) → run_id mapping
            self.run_id_mapping[(i, run_date, run_start_time)] = run_id  # Now storing row index for lookup

            for j, value in enumerate([
                run_date, run_start_time, run[5], run[6], run[7], run[8],
                run[9] if run[9] is not None else "N/A",
                run[10] if run[10] is not None else "N/A",
                run[11] if run[11] is not None else "N/A",
                run[12] if run[12] is not None else "N/A",
                run[13], run[14], run[15], run[16], run[17]
            ]):
                item = NumericTableWidgetItem(value)

                if j in right_align_columns:
                    item.setTextAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
                else:
                    item.setTextAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)

                self.tableWidget.setItem(i, j, item)


    def handle_cell_clicked(self, row, column):
        """Opens the details window for the clicked run."""
        try:
            date = self.tableWidget.item(row, 0).text()
            start_time = self.tableWidget.item(row, 1).text()

            # Lookup run_id by (row, date, start_time)
            run_id = self.run_id_mapping.get((row, date, start_time))

            if not run_id:
                print(f"Error: No matching run_id for Date: {date}, Start Time: {start_time}")
                return

            # Prevent opening multiple detail windows
            if hasattr(self, "details_window") and self.details_window and self.details_window.isVisible():
                return

            # Fetch run details from the DB
            run_data = get_run_by_id(run_id)

            if run_data:
                self.details_window = RunDetailsWindow(run_data)
                self.details_window.exec()

                # Ensure reference is cleared after closing
                self.details_window = None

        except Exception as e:
            print(f"Error opening details window: {e}")

    def upload_tcx_file(self):
        file_path, _ = QFileDialog.getOpenFileName(self, "Select TCX File", "", "TCX Files (*.tcx)")
        if file_path:
            self.process_tcx_file(file_path)
            QMessageBox.information(self, "Processing Complete", "TCX file processed.")
            self.load_years()

    def process_tcx_file(self, file_path):
        df, activity_type = parse_tcx(file_path)
        df = convert_to_utm(df)
        df = calculate_distance(df)
        df, avg_pace, fastest_pace, slowest_pace = calculate_pace(df, activity_type)
        pause_time = detect_pauses(df)
        avg_steps, total_steps = calculate_steps(df)

        base_name = os.path.basename(file_path).replace(".tcx", "")
        date, time = base_name.split("_")[:2]  # Extract date and start time
        track_img = os.path.join(IMG_DIR, f"{base_name}_track.png")
        elevation_img = os.path.join(IMG_DIR, f"{base_name}_elevation.svg")
        map_html = os.path.join(IMG_DIR, f"{base_name}_map.html")

        if not os.path.exists(track_img):
            plot_track(df, track_img)
        if not os.path.exists(elevation_img):
            plot_elevation(df, elevation_img)
        if not os.path.exists(map_html):
            plot_activity_map(df, map_html)

        year, month = date.split("-")[:2]

        run_data = compute_run_db_data(df, base_name, year, month, avg_steps, total_steps, avg_pace, fastest_pace, slowest_pace, pause_time, activity_type)
        insert_run(run_data, track_img, elevation_img, map_html)


# ==========================
# 🌟 Improved Splash Screen
# ==========================
if __name__ == "__main__":
    app = QApplication(sys.argv)

    # Load splash screen
    splash_pixmap = QPixmap("splash_png.jpg")
    splash = QSplashScreen(splash_pixmap)
    splash.show()

    # Update splash message
    def update_splash(message):
        splash.showMessage(message, Qt.AlignmentFlag.AlignBottom | Qt.AlignmentFlag.AlignHCenter)

    # Step 1: Initialize Database
    update_splash("Initializing database...")
    initialize_database()

    # Step 2: Load UI after a short delay (allowing splash to be visible)
    def start_main_app():
        global window  # Ensure window persists
        update_splash("Loading user interface...")
        window = RunningDataApp()

        update_splash("Finalizing startup...")
        QTimer.singleShot(500, splash.close)  # Give 500ms for splash to fade
        window.showMaximized()

    # Start the main window **after** a short delay, allowing splash visibility
    QTimer.singleShot(1000, start_main_app)  # Delay startup slightly

    sys.exit(app.exec())