import sys
import os
from PyQt6.QtWidgets import (
    QApplication, QWidget, QVBoxLayout, QPushButton, QFileDialog, QLabel,
    QTableWidget, QTableWidgetItem, QComboBox, QMessageBox, QHeaderView, QSplashScreen, QHBoxLayout
)
from PyQt6.QtGui import QPixmap, QIcon
from PyQt6.QtCore import Qt, QTimer

from processing.compute_statistics import compute_run_db_data
from processing.data_processing import convert_to_utm, calculate_distance, calculate_pace, detect_pauses, \
    calculate_steps
from processing.parse_tcx import parse_tcx
from processing.system_settings import ViewMode, SortOrder, mapActivityTypes
from processing.visualization import plot_track, plot_elevation, plot_activity_map

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from ui.main_menu import MenuBar
from ui.table_builder import TableBuilder
from ui.window_run_details import RunDetailsWindow
from translations import _

# Directories
IMG_DIR = os.path.expanduser("~/RunningData/images")
if not os.path.exists(IMG_DIR):
    os.makedirs(IMG_DIR)

MEDIA_DIR = os.path.expanduser("~/RunningData/media")
if not os.path.exists(MEDIA_DIR):
    os.makedirs(MEDIA_DIR)


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
    def __init__(self, db_handler: DatabaseHandler):
        super().__init__()
        self.details_window = None
        self.tableWidget = None
        self.db = db_handler
        self.setWindowIcon(QIcon("icon.icns"))
        self.view_mode = ViewMode.ALL
        self.sort_field = "date_time"
        self.sort_direction = SortOrder.DESC
        self.view_buttons = {}
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout()

        menu_bar = MenuBar(self)
        layout.addWidget(menu_bar)

        view_mode_layout = QHBoxLayout()

        self.view_buttons[ViewMode.ALL] = QPushButton(_("All"))
        self.view_buttons[ViewMode.RUN] = QPushButton(_("Runs"))
        self.view_buttons[ViewMode.CYCLE] = QPushButton(_("Rides"))
        self.view_buttons[ViewMode.WALK] = QPushButton(_("Walks"))

        self.view_buttons[ViewMode.ALL].clicked.connect(lambda: self.set_active_view(ViewMode.ALL))
        self.view_buttons[ViewMode.RUN].clicked.connect(lambda: self.set_active_view(ViewMode.RUN))
        self.view_buttons[ViewMode.CYCLE].clicked.connect(lambda: self.set_active_view(ViewMode.CYCLE))
        self.view_buttons[ViewMode.WALK].clicked.connect(lambda: self.set_active_view(ViewMode.WALK))

        for button in self.view_buttons.values():
            button.setCheckable(True)
            button.setAutoExclusive(True)
            view_mode_layout.addWidget(button)

        layout.addLayout(view_mode_layout)
        
        self.tableWidget = QTableWidget()
        layout.addWidget(self.tableWidget)

        self.setLayout(layout)
        self.setWindowTitle("Running Mate")
        self.load_activities()
        self.update_button()

    def set_active_view(self, view_mode):
        """Switch the view and update button styles"""
        self.view_mode = view_mode
        print(f"Switching to view: {view_mode}")

        if view_mode == ViewMode.ALL:
            self.load_activities()
        elif view_mode == ViewMode.RUN:
            self.load_runs()

        self.update_button()

    def update_button(self):
        """Update button styles to highlight the active one"""
        self.view_buttons[self.view_mode].setChecked(True)

    def center_window(self):
        """Centers the window on the primary screen."""
        screen = QApplication.primaryScreen().geometry()
        window_rect = self.geometry()

        x = (screen.width() - window_rect.width()) // 2
        y = (screen.height() - window_rect.height()) // 2

        self.move(x, y)

    def load_activities(self, sort_field="date_time"):
        """
        Fetch activities from the database and display them in the table.
        :param sort_field: Column name to sort by (default "activities.date")
        """
        activities = self.db.fetch_activities(sort_field=sort_field, sort_direction=self.sort_direction)
        TableBuilder.setup_table(self.tableWidget, ViewMode.ALL, activities, self)
        TableBuilder.update_header_styles(self.tableWidget, ViewMode.ALL, sort_field, self.sort_direction)

    def load_runs(self, sort_field="date_time"):
        runs = self.db.fetch_runs(sort_field=sort_field)
        TableBuilder.setup_table(self.tableWidget, ViewMode.RUN, runs, self)
        TableBuilder.update_header_styles(self.tableWidget, ViewMode.RUN, sort_field, self.sort_direction)

    def load_detail(self, data):
        if data['activity_type'] == 'Running':
            run_data = self.db.fetch_run_by_activity_id(data['activity_id'])
            self.details_window = RunDetailsWindow(run_data, MEDIA_DIR, self.db)
            self.details_window.exec()
            self.details_window = None

    def get_sorting_direction(self, view_mode=ViewMode.ALL):
        if view_mode != self.view_mode:
            return SortOrder.DESC

        if self.sort_direction == SortOrder.ASC:
            return SortOrder.DESC
        return SortOrder.ASC

    def sort_by_column(self, activity_type, column):
        activity_type = mapActivityTypes(activity_type)
        self.sort_direction = self.get_sorting_direction(activity_type)
        if activity_type == ViewMode.ALL:
            self.load_activities(sort_field=column)
        elif activity_type == ViewMode.RUN:
            self.load_runs(sort_field=column)

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
            run_data = self.db.get_run_by_id(run_id)

            if run_data:
                self.details_window = RunDetailsWindow(run_data, MEDIA_DIR, self.db)
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

        run_data = compute_run_db_data(df, base_name, year, month, avg_steps, total_steps, avg_pace, fastest_pace,
                                       slowest_pace, pause_time, activity_type)
        self.db.insert_run(run_data, track_img, elevation_img, map_html)


if __name__ == "__main__":
    app = QApplication(sys.argv)

    # Load splash screen
    splash_pixmap = QPixmap("splash_screen.png")
    splash = QSplashScreen(splash_pixmap)
    splash.show()


    # Update splash message
    def update_splash(message):
        splash.showMessage(message, Qt.AlignmentFlag.AlignBottom | Qt.AlignmentFlag.AlignHCenter)


    # Step 1: Initialize Database
    update_splash(_("Initializing database..."))

    db_handler = DatabaseHandler()
    apply_migrations(db_handler)


    # Step 2: Load UI after a short delay (allowing splash to be visible)
    def start_main_app():
        global window  # Ensure window persists
        update_splash(_("Loading user interface..."))
        window = RunningDataApp(db_handler)

        update_splash(_("Finalizing startup..."))
        QTimer.singleShot(500, splash.close)  # Give 500ms for splash to fade
        window.showMaximized()


    # Start the main window **after** a short delay, allowing splash visibility
    QTimer.singleShot(1000, start_main_app)  # Delay startup slightly

    sys.exit(app.exec())
