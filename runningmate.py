import os
import sys

from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QIcon, QPixmap
from PyQt6.QtWidgets import (
    QApplication,
    QFrame,
    QHBoxLayout,
    QLabel,
    QMessageBox,
    QPushButton,
    QSizePolicy,
    QSplashScreen,
    QSplitter,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from importer.file.tcx_file import TcxFileImporter
from importer.garmin.garmin import garmin_connect_login
from processing.plot_heatmap import PlotHeatmap
from processing.system_settings import SortOrder, ViewMode, mapActivityTypes
from ui.info_card import InfoCard
from ui.main_menu import MenuBar
from ui.side_bar import Sidebar
from ui.table_builder import TableBuilder
from ui.window_garmin_sync import GarminSyncWindow
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
        self.heatmap = None
        self.tool_bar = None
        self.page_label = None
        self.sync_window = None
        self.details_window = None
        self.tableWidget = None
        self.db = db_handler
        self.view_mode = ViewMode.ALL
        self.nav_bar = None
        self.sort_field = "date_time"
        self.sort_direction = SortOrder.DESC
        self.current_page = 0
        self.page_size = 25
        self.offset = 0
        self.init_ui()

    def init_ui(self):
        nav_buttons = {
            ViewMode.ALL: ("gallery-view.svg", "All"),
            ViewMode.RUN: ("run-fill.svg", "Runs"),
            ViewMode.CYCLE: ("riding-line.svg", "Rides"),
            ViewMode.WALK: ("walk-fill.svg", "Walks"),
        }

        tool_buttons = {
            "search": ("search-line.svg", "Search"),
            "user": ("user-line.svg", "User"),
            "settings": ("settings-2-line.svg", "Settings"),
        }

        # 🔹 Main Vertical Layout
        main_layout = QVBoxLayout()

        menu_bar = MenuBar(self)
        main_layout.addWidget(menu_bar)

        splitter = QSplitter(Qt.Orientation.Horizontal)

        self.left_widget = QWidget()
        self.left_layout = QVBoxLayout(self.left_widget)
        self.left_layout.setContentsMargins(0, 0, 0, 0)

        self.nav_bar = Sidebar(nav_buttons, self.left_widget)
        self.nav_bar.action_triggered.connect(self.set_active_view)
        self.left_layout.addWidget(self.nav_bar)

        self.left_layout.addStretch()

        self.tool_bar = Sidebar(tool_buttons, self.left_widget)
        self.tool_bar.setFixedHeight(150)
        self.tool_bar.action_triggered.connect(self.trigger_tool_action)
        self.left_layout.addWidget(self.tool_bar)

        self.left_widget.setMinimumWidth(50)  # Ensures it's always at least 80px
        self.left_widget.setMaximumWidth(200)  # Optional max width
        self.left_widget.setLayout(self.left_layout)

        splitter.addWidget(self.left_widget)

        self.center_widget = QWidget()
        self.center_layout = QVBoxLayout(self.center_widget)

        top_row = QHBoxLayout()

        self.activities_card = InfoCard(self.db, metric="activities")
        self.distance_card = InfoCard(self.db, metric="distance")
        self.duration_card = InfoCard(self.db, metric="duration")
        self.elevation_card = InfoCard(self.db, metric="elevation")

        self.heatmap = PlotHeatmap(self.db)
        self.heatmap_widget = QWidget()
        self.heatmap_layout = QVBoxLayout(self.heatmap_widget)
        self.heatmap_layout.setContentsMargins(0, 0, 0, 0)
        self.heatmap_widget.setStyleSheet("background-color: transparent")
        self.heatmap_widget.setFixedSize(624, 84)

        self.heatmap_label = QLabel()
        self.heatmap_label.setScaledContents(True)  # Ensure image fits in the widget
        self.heatmap_layout.addWidget(self.heatmap_label)

        self.heatmap_widget.setLayout(self.heatmap_layout)

        heatmap_image = self.heatmap.get_heatmap()
        if heatmap_image:
            pixmap = QPixmap(heatmap_image)
            self.heatmap_label.setPixmap(pixmap)

        top_row.addWidget(self.activities_card)
        top_row.addWidget(self.distance_card)
        top_row.addWidget(self.duration_card)
        top_row.addWidget(self.elevation_card)
        top_row.addStretch()
        top_row.addWidget(self.heatmap_widget, 0)

        self.center_layout.addLayout(top_row)

        self.tableWidget = QTableWidget()
        self.tableWidget.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        self.center_layout.addWidget(self.tableWidget, 1)
        self.center_layout.addStretch()

        pagination_layout = QHBoxLayout()
        self.prev_button = QPushButton(_("Previous"))
        self.next_button = QPushButton(_("Next"))
        self.page_label = QLabel(f"{_('Page')} 1")

        self.prev_button.clicked.connect(self.previous_page)
        self.next_button.clicked.connect(self.next_page)

        pagination_layout.addWidget(self.prev_button)
        pagination_layout.addWidget(
            self.page_label, alignment=Qt.AlignmentFlag.AlignCenter
        )
        pagination_layout.addWidget(self.next_button)

        self.center_layout.addLayout(pagination_layout)

        self.center_widget.setLayout(self.center_layout)
        splitter.addWidget(self.center_widget)

        # ───────────────────────────────────────────
        # 🔹 RIGHT PANEL (INFO CARDS - FIXED)
        # ───────────────────────────────────────────
        self.right_widget = QWidget()
        self.right_layout = QVBoxLayout(self.right_widget)

        self.card_1 = QLabel(_("Card 1"))
        self.card_1.setFrameShape(QFrame.Shape.Box)
        self.card_2 = QLabel(_("Card 2"))
        self.card_2.setFrameShape(QFrame.Shape.Box)
        self.card_3 = QLabel(_("Card 3"))
        self.card_3.setFrameShape(QFrame.Shape.Box)

        self.right_layout.addWidget(self.card_1)
        self.right_layout.addWidget(self.card_2)
        self.right_layout.addWidget(self.card_3)
        self.right_layout.addStretch()

        self.right_widget.setLayout(self.right_layout)

        # ✅ Add Right Panel to Splitter (Always Visible & FIXED WIDTH)
        self.right_widget.setFixedWidth(250)  # FIXED WIDTH so it cannot be resized
        splitter.addWidget(self.right_widget)

        # ✅ Set Default Split Sizes: Left (Small), Center (Large), Right (Fixed)
        splitter.setSizes(
            [50, 800, 250]
        )  # Left collapsed by default, center large, right fixed

        # ✅ Prevent RIGHT PANEL from being resized
        splitter.setCollapsible(0, False)  # Left Panel is NOT collapsible
        splitter.setCollapsible(1, False)  # Center Panel is NOT collapsible
        splitter.setCollapsible(2, False)  # Right Panel is NOT collapsible

        splitter.setStretchFactor(
            0, 0
        )  # Left panel remains at min width unless expanded
        splitter.setStretchFactor(1, 1)  # Center panel takes priority in resizing
        splitter.setStretchFactor(2, 0)  # Right panel stays FIXED

        splitter.setStyleSheet(
            """
            QSplitter::handle {
                background-color: #000;  /* Light Gray */
                width: 1px;  /* Make divider thicker */
            }
        """
        )

        main_layout.addWidget(splitter)

        self.setLayout(main_layout)
        self.setWindowTitle("RunningMate")

        # ✅ Initialize Heatmap
        # self.heatmap = PlotHeatmap(self.db, self.heatmap_layout)
        # self.heatmap.get_heatmap()

        self.load_activities()
        self.update_button()
        self.update_pagination()

    def set_active_view(self, view_mode):
        """Switch the view and update button styles"""
        if view_mode == self.view_mode:  # ✅ Prevent unnecessary recursive calls
            return
        self.view_mode = view_mode
        self.offset = 0
        self.current_page = 0

        self.trigger_load()
        self.update_button()
        self.update_pagination()
        self.update_heatmap(view_mode)

    def update_heatmap(self, view_mode):
        heatmap_image = self.heatmap.get_heatmap(activity_type=view_mode)
        if heatmap_image:
            pixmap = QPixmap(heatmap_image)
            self.heatmap_label.setPixmap(pixmap)

    def trigger_tool_action(self, action):
        print(action)

    def trigger_load(self):
        if self.view_mode == ViewMode.RUN:
            self.load_runs()
        elif self.view_mode == ViewMode.CYCLE:
            self.load_rides()
        elif self.view_mode == ViewMode.WALK:
            self.load_walks()
        else:
            self.load_activities()

    def update_button(self):
        if hasattr(self, "nav_bar") and self.nav_bar:  # Ensure nav_bar exists
            self.nav_bar.set_active_action(self.view_mode)

    def center_window(self):
        """Centers the window on the primary screen."""
        screen = QApplication.primaryScreen().geometry()
        window_rect = self.geometry()

        x = (screen.width() - window_rect.width()) // 2
        y = (screen.height() - window_rect.height()) // 2

        self.move(x, y)

    def update_pagination(self):
        total_rows = self.db.get_total_activity_count(self.view_mode)
        total_pages = max(1, (total_rows + self.page_size - 1) // self.page_size)

        self.prev_button.setEnabled(self.current_page > 0)
        self.next_button.setEnabled(self.current_page < total_pages - 1)
        self.page_label.setText(f"{_('Page')} {self.current_page + 1} / {total_pages}")

    def previous_page(self):
        if self.current_page > 0:
            self.current_page -= 1
            self.update_view()

    def next_page(self):
        total_rows = self.db.get_total_activity_count(self.view_mode)
        total_pages = max(1, (total_rows + self.page_size - 1) // self.page_size)
        if self.current_page < total_pages - 1:
            self.current_page += 1
            self.update_view()

    def update_view(self):
        self.offset = self.current_page * self.page_size
        self.trigger_load()
        self.update_pagination()

    def load_activities(self):
        activities = self.db.fetch_activities(
            start=self.offset,
            sort_field=self.sort_field,
            limit=self.page_size,
            sort_direction=self.sort_direction,
        )
        TableBuilder.setup_table(self.tableWidget, ViewMode.ALL, activities, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.ALL, self.sort_field, self.sort_direction
        )

    def load_runs(self):
        runs = self.db.fetch_runs(
            start=self.offset,
            sort_field=self.sort_field,
            limit=self.page_size,
            sort_direction=self.sort_direction,
        )
        TableBuilder.setup_table(self.tableWidget, ViewMode.RUN, runs, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.RUN, self.sort_field, self.sort_direction
        )

    def load_walks(self):
        runs = self.db.fetch_walks(
            start=self.offset,
            sort_field=self.sort_field,
            limit=self.page_size,
            sort_direction=self.sort_direction,
        )
        TableBuilder.setup_table(self.tableWidget, ViewMode.WALK, runs, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.WALK, self.sort_field, self.sort_direction
        )

    def load_rides(self):
        runs = self.db.fetch_rides(
            start=self.offset,
            sort_field=self.sort_field,
            limit=self.page_size,
            sort_direction=self.sort_direction,
        )
        TableBuilder.setup_table(self.tableWidget, ViewMode.CYCLE, runs, self)
        TableBuilder.update_header_styles(
            self.tableWidget, ViewMode.CYCLE, self.sort_field, self.sort_direction
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
        self.offset = 0
        self.current_page = 0
        activity_type = mapActivityTypes(activity_type)
        self.sort_direction = self.get_sorting_direction(activity_type)
        self.sort_field = column
        self.trigger_load()
        self.update_pagination()

    def garmin_connect(self):
        client = garmin_connect_login()
        if client:
            self.sync_window = GarminSyncWindow(client, FILE_DIR, IMG_DIR, self.db)
            self.sync_window.exec()
            self.sync_window = None

    def upload_tcx_file(self):
        importer = TcxFileImporter(FILE_DIR, IMG_DIR, self.db, self)
        importer.by_upload()
        self.processed_msg()
        self.set_active_view(self.view_mode)

    def refresh_entry(self, activity_id):
        importer = TcxFileImporter(FILE_DIR, IMG_DIR, self.db, self)
        imported = importer.by_activity(activity_id)
        if imported:
            self.processed_msg(activity_id)
            self.set_active_view(self.view_mode)

    def delete_entry(self, activity_id):
        """Delete the selected activity after confirmation."""

        reply = QMessageBox.question(
            self,
            "Delete Confirmation",
            f"Are you sure you want to delete activity {activity_id}?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )

        if reply == QMessageBox.StandardButton.Yes:
            self.db.delete_activity(activity_id)
            self.set_active_view(self.view_mode)

    def processed_msg(self, activity_id=None):
        msg = QMessageBox()
        msg.setIcon(QMessageBox.Icon.Information)
        msg.setWindowTitle("Success")
        if activity_id:
            msg.setText(f"Activity {activity_id} has been processed.")
        else:
            msg.setText("The file has been imported.")
        msg.setStandardButtons(QMessageBox.StandardButton.Ok)
        msg.exec()


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
        window.setWindowIcon(QIcon("app-icon.icns"))
        app.setWindowIcon(QIcon("app-icon.icns"))
        app.setOrganizationName("WebAndApp")
        app.setApplicationName("RunningMate")
        app.setApplicationVersion("0.3")
        window.showMaximized()

    # Start the main window **after** a short delay, allowing splash visibility
    QTimer.singleShot(1000, start_main_app)  # Delay startup slightly

    sys.exit(app.exec())
