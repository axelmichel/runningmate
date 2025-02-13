import os
import sys

from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QIcon, QPixmap
from PyQt6.QtWidgets import (
    QApplication,
    QHBoxLayout,
    QPushButton,
    QSplashScreen,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from importer.file.tcx_file import TcxFileImporter
from processing.system_settings import SortOrder, ViewMode, mapActivityTypes
from ui.main_menu import MenuBar
from ui.table_builder import TableBuilder
from ui.window_run_details import RunDetailsWindow
from utils.translations import _

# Directories
IMG_DIR = os.path.expanduser("~/RunningData/images")
if not os.path.exists(IMG_DIR):
    os.makedirs(IMG_DIR)

MEDIA_DIR = os.path.expanduser("~/RunningData/media")
if not os.path.exists(MEDIA_DIR):
    os.makedirs(MEDIA_DIR)

FILE_DIR = os.path.expanduser("~/RunningData/imports")
if not os.path.exists(FILE_DIR):
    os.makedirs(FILE_DIR)


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

        self.view_buttons[ViewMode.ALL].clicked.connect(
            lambda: self.set_active_view(ViewMode.ALL)
        )
        self.view_buttons[ViewMode.RUN].clicked.connect(
            lambda: self.set_active_view(ViewMode.RUN)
        )
        self.view_buttons[ViewMode.CYCLE].clicked.connect(
            lambda: self.set_active_view(ViewMode.CYCLE)
        )
        self.view_buttons[ViewMode.WALK].clicked.connect(
            lambda: self.set_active_view(ViewMode.WALK)
        )

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

        if view_mode == ViewMode.RUN:
            self.load_runs()
        elif view_mode == ViewMode.CYCLE:
            self.load_rides()
        elif view_mode == ViewMode.WALK:
            self.load_walks()
        else:
            self.load_activities()

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
        activities = self.db.fetch_activities(
            sort_field=sort_field, sort_direction=self.sort_direction
        )
        TableBuilder.setup_table(self.tableWidget, ViewMode.ALL, activities, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.ALL, sort_field, self.sort_direction
        )

    def load_runs(self, sort_field="date_time"):
        runs = self.db.fetch_runs(sort_field=sort_field)
        TableBuilder.setup_table(self.tableWidget, ViewMode.RUN, runs, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.RUN, sort_field, self.sort_direction
        )

    def load_walks(self, sort_field="date_time"):
        runs = self.db.fetch_walks(sort_field=sort_field)
        TableBuilder.setup_table(self.tableWidget, ViewMode.WALK, runs, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.WALK, sort_field, self.sort_direction
        )

    def load_rides(self, sort_field="date_time"):
        runs = self.db.fetch_rides(sort_field=sort_field)
        TableBuilder.setup_table(self.tableWidget, ViewMode.CYCLE, runs, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.CYCLE, sort_field, self.sort_direction
        )

    def load_detail(self, data):
        activity_type = mapActivityTypes(data["activity_type"])
        if activity_type == ViewMode.RUN:
            data = self.db.fetch_run_by_activity_id(data["activity_id"])
            self.details_window = RunDetailsWindow(data, MEDIA_DIR, self.db)
            self.details_window.exec()
            self.details_window = None
        elif activity_type == ViewMode.WALK:
            data = self.db.fetch_walk_by_activity_id(data["activity_id"])
            self.details_window = RunDetailsWindow(data, MEDIA_DIR, self.db)
            self.details_window.exec()
            self.details_window = None
        elif activity_type == ViewMode.CYCLE:
            data = self.db.fetch_ride_by_activity_id(data["activity_id"])
            self.details_window = RunDetailsWindow(data, MEDIA_DIR, self.db)
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

    def upload_tcx_file(self):
        importer = TcxFileImporter(FILE_DIR, IMG_DIR, self.db)
        importer.upload()
        self.set_active_view(self.view_mode)


if __name__ == "__main__":
    app = QApplication(sys.argv)

    # Load splash screen
    splash_pixmap = QPixmap("splash_screen.png")
    splash = QSplashScreen(splash_pixmap)
    splash.show()

    # Update splash message
    def update_splash(message):
        splash.showMessage(
            message, Qt.AlignmentFlag.AlignBottom | Qt.AlignmentFlag.AlignHCenter
        )

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
