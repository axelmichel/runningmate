import os
import sys
import webbrowser

from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QIcon, QPixmap
from PyQt6.QtWidgets import (
    QApplication,
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
from database.user_settings import UserSettings
from importer.file.tcx_file import TcxFileImporter
from importer.garmin.garmin import garmin_connect_login
from processing.activity_info import ActivityInfo
from processing.best_performances import BestSegmentFinder
from processing.plot_heatmap import PlotHeatmap
from processing.system_settings import SortOrder, ViewMode, mapActivityTypes
from ui.activity_widget import ActivityWidget
from ui.best_performances_widget import BestPerformanceWidget
from ui.info_card import InfoCard
from ui.main_menu import MenuBar
from ui.side_bar import Sidebar
from ui.table_builder import TableBuilder
from ui.window_garmin_sync import GarminSyncWindow
from ui.window_run_details import RunDetailsWindow
from ui.window_user_settings import UserSettingsWindow
from utils.app_mode import is_dark_mode
from utils.resource_path import resource_path
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
        self.activity_widget = None
        self.right_widget = None
        self.heatmap = None
        self.tool_bar = None
        self.page_label = None
        self.sync_window = None
        self.details_window = None
        self.user_window = None
        self.tableWidget = None
        self.db = db_handler
        self.userSettings = UserSettings(self.db)
        self.view_mode = ViewMode.ALL
        self.nav_bar = None
        self.sort_field = "date_time"
        self.sort_direction = SortOrder.DESC
        self.current_page = 0
        self.page_size = 25
        self.offset = 0
        self.activity_info_handler = ActivityInfo(self.db, IMG_DIR)
        self.activity_performance_widget = None
        self.best_performance_handler = BestSegmentFinder(self.db)
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

        icon_folder = "light" if is_dark_mode() else "dark"
        pagination_layout = QHBoxLayout()

        # Previous Button (⬅ arrow-left-line)
        self.prev_button = QPushButton()
        self.prev_button.setIcon(
            QIcon(resource_path(f"icons/{icon_folder}/arrow-left-line.svg"))
        )
        self.prev_button.setToolTip(_("Previous Page"))
        self.prev_button.clicked.connect(self.previous_page)

        # Next Button (➡ arrow-right-line)
        self.next_button = QPushButton()
        self.next_button.setIcon(
            QIcon(resource_path(f"icons/{icon_folder}/arrow-right-line.svg"))
        )
        self.next_button.setToolTip(_("Next Page"))
        self.next_button.clicked.connect(self.next_page)

        # Page Label (Current Page x / Total Pages)
        self.page_label = QLabel()
        self.page_label.setAlignment(Qt.AlignmentFlag.AlignCenter)

        self.page_label = QLabel()
        self.page_label.setAlignment(Qt.AlignmentFlag.AlignCenter)

        self.items_info_label = QLabel()
        self.items_info_label.setAlignment(Qt.AlignmentFlag.AlignCenter)

        # Add widgets to layout
        pagination_layout.addWidget(self.prev_button)
        pagination_layout.addWidget(self.page_label)
        pagination_layout.addWidget(self.next_button)

        self.center_layout.addLayout(pagination_layout)

        self.center_widget.setLayout(self.center_layout)
        splitter.addWidget(self.center_widget)

        # ───────────────────────────────────────────
        # 🔹 RIGHT PANEL (INFO CARDS - FIXED)
        # ───────────────────────────────────────────
        # Create the right panel layout
        self.right_widget = QWidget()
        self.right_layout = QVBoxLayout(self.right_widget)

        # Placeholder for ActivityWidget (initialized empty)
        self.activity_widget = None
        self.activity_performance_widget = None

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

        self.load_activities()
        self.update_button()
        self.update_pagination()
        self.update_activity_view()

    def set_active_view(self, view_mode):
        """Switch the view and update button styles"""
        if view_mode == self.view_mode:
            return
        self.view_mode = view_mode
        self.offset = 0
        self.current_page = 0

        self.trigger_load()
        self.update_infoCards()
        self.update_button()
        self.update_pagination()
        self.update_heatmap(view_mode)
        self.update_activity_view(view_mode)

    def update_activity_view(self, activity_type=ViewMode.ALL, activity_id=None):
        """
        Update the right panel with the new activity information.

        :param activity_id: Optional[int]
            The selected activity ID (if provided).
        :param activity_type: Optional[ViewMode]
            The selected activity type (if provided).
        """
        # Fetch the latest or specific activity info
        activity_data = self.activity_info_handler.get_activity_info(
            activity_type, activity_id
        )

        if activity_data is None:
            print("No activity found!")
            return  # Avoid errors if no data is found

        best_performance_data = self.best_performance_handler.get_best_segments(
            activity_data["id"],
            activity_data["category"],
        )

        if self.activity_widget:
            self.right_layout.removeWidget(self.activity_widget)
            self.activity_widget.deleteLater()
            self.activity_widget = None

        if self.activity_performance_widget:
            self.right_layout.removeWidget(self.activity_performance_widget)
            self.activity_performance_widget.deleteLater()
            self.activity_performance_widget = None

        self.activity_widget = ActivityWidget(activity_data)
        self.right_layout.insertWidget(0, self.activity_widget)

        if best_performance_data:
            self.activity_performance_widget = BestPerformanceWidget(
                best_performance_data
            )
            self.right_layout.insertWidget(1, self.activity_performance_widget)

    def update_heatmap(self, view_mode):
        heatmap_image = self.heatmap.get_heatmap(activity_type=view_mode)
        if heatmap_image:
            pixmap = QPixmap(heatmap_image)
            self.heatmap_label.setPixmap(pixmap)

    def trigger_tool_action(self, action):
        if action == "user":
            self.user_window = UserSettingsWindow(self.userSettings, self)
            self.user_window.exec()
            self.user_window = None

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

        start_item = self.current_page * self.page_size + 1
        end_item = min((self.current_page + 1) * self.page_size, total_rows)

        self.prev_button.setEnabled(self.current_page > 0)
        self.next_button.setEnabled(self.current_page < total_pages - 1)
        self.page_label.setText(
            f"{_('Page')} {self.current_page + 1} / {total_pages} | {_('Showing')} {start_item}-{end_item} {_('of')} {total_rows}"
        )

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

    def update_infoCards(self):
        self.activities_card.update_info(self.view_mode)
        self.distance_card.update_info(self.view_mode)
        self.duration_card.update_info(self.view_mode)
        self.elevation_card.update_info(self.view_mode)

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

    def load_detail(self, activity_id):
        self.update_activity_view(None, activity_id)

    def open_detail(self, activity_id, activity_type):
        if activity_type == ViewMode.RUN:
            data = self.db.fetch_run_by_activity_id(activity_id)
            self.details_window = RunDetailsWindow(data, MEDIA_DIR, self.db)
            self.details_window.exec()
            self.details_window = None
        elif activity_type == ViewMode.WALK:
            data = self.db.fetch_walk_by_activity_id(activity_id)
            self.details_window = RunDetailsWindow(data, MEDIA_DIR, self.db)
            self.details_window.exec()
            self.details_window = None
        elif activity_type == ViewMode.CYCLE:
            data = self.db.fetch_ride_by_activity_id(activity_id)
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
        uploaded = importer.by_upload()
        if uploaded:
            self.heatmap.clear_heatmaps()
            self.processed_msg()
            self.trigger_load()
            self.update_infoCards()
            self.update_heatmap(self.view_mode)
            self.update_activity_view(self.view_mode)

    def refresh_entry(self, activity_id):
        importer = TcxFileImporter(FILE_DIR, IMG_DIR, self.db, self)
        imported = importer.by_activity(activity_id)
        if imported:
            self.processed_msg(activity_id)
            self.trigger_load()
            self.update_infoCards()
            self.update_heatmap(self.view_mode)
            self.update_activity_view(self.view_mode)

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
            self.trigger_load()
            self.update_infoCards()
            self.update_heatmap(self.view_mode)
            self.update_activity_view(self.view_mode)

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

    def show_about(self):
        """Show splash screen when menu item is clicked"""
        splash_pixmap = QPixmap("splash_screen.png")  # Replace with your splash image
        self.splash = QSplashScreen(splash_pixmap, Qt.WindowType.WindowStaysOnTopHint)
        self.splash.show()

        # Simulate loading process (2 seconds)
        QTimer.singleShot(2000, self.hide_splash_screen)

    def online_help(self):
        webbrowser.open(
            "https://axelmichel.github.io/runningmate/"
        )  # Change to your desired URL

    def hide_splash_screen(self):
        """Hide the splash screen after loading"""
        self.splash.close()


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
