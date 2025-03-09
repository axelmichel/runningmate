from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QDialog,
    QHBoxLayout,
    QLabel,
    QSplitter,
    QStackedWidget,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from processing.system_settings import ViewMode
from ui.side_bar import Sidebar
from utils.translations import _


class DialogDetail(QDialog):
    def __init__(self, activity_id, activity_type, media_dir, db_handler: DatabaseHandler, parent=None):
        super().__init__()
        self.pages = None
        self.map_page = None
        self.general_page = None
        self.effect_page = None
        self.zones_page = None
        self.nav_bar = None
        self.left_layout = None
        self.left_widget = None
        self.parent = parent
        self.db = db_handler
        self.media_dir = media_dir
        self.activity = self.load_activity(activity_id, activity_type)

    def init_ui(self):
        nav_buttons = {
            "general": ("user-line.svg", "General"),
            "map": ("user-line.svg", "Map"),
            "effect": ("footprint-fill.svg", "Effect"),
            "zones": ("heart-pulse-line.svg", "Heart Rate Zones"),
        }

        self.setWindowTitle(_("Activity Details"))
        self.setGeometry(100, 100, 800, 500)

        main_layout = QHBoxLayout()

        splitter = QSplitter(Qt.Orientation.Horizontal)
        self.left_widget = QWidget()
        self.left_layout = QVBoxLayout(self.left_widget)
        self.left_layout.setContentsMargins(0, 0, 0, 0)
        self.left_widget.setMinimumWidth(50)
        self.left_widget.setMaximumWidth(200)

        self.nav_bar = Sidebar(nav_buttons, self.left_widget)
        self.nav_bar.action_triggered.connect(self.set_active_page)
        self.left_layout.addWidget(self.nav_bar)
        self.left_layout.addStretch()
        splitter.addWidget(self.left_widget)

        self.pages = QStackedWidget()

        self.general_page = self.create_general_page()
        self.map_page = self.create_map_page()
        self.effect_page = self.create_effect_page()
        self.zones_page = self.create_zones_page()

        self.pages.addWidget(self.general_page)
        self.pages.addWidget(self.map_page)
        self.pages.addWidget(self.effect_page)
        self.pages.addWidget(self.zones_page)

        splitter.addWidget(self.pages)
        splitter.setSizes([50, 750])

        splitter.setCollapsible(0, False)  # Left Panel is NOT collapsible
        splitter.setCollapsible(1, False)  # Center Panel is NOT collapsible

        splitter.setStretchFactor(
            0, 0
        )  # Left panel remains at min width unless expanded
        splitter.setStretchFactor(1, 1)  # Center panel takes priority in resizing

        splitter.setStyleSheet(
            """
            QSplitter::handle {
                background-color: #000;  /* Light Gray */
                width: 1px;  /* Make divider thicker */
            }
        """
        )

        main_layout.addWidget(splitter)
        self.nav_bar.set_active_action("general")
        self.setLayout(main_layout)

    def load_activity(self, activity_id, activity_type):
        data = None
        if activity_type == ViewMode.RUN:
            data = self.db.fetch_run_by_activity_id(activity_id)
        elif activity_type == ViewMode.WALK:
            data = self.db.fetch_walk_by_activity_id(activity_id)
        elif activity_type == ViewMode.CYCLE:
            data = self.db.fetch_ride_by_activity_id(activity_id)
        return data

    def set_active_page(self, page) -> None:
        if page == "general":
            self.pages.setCurrentWidget(self.general_page)
        elif page == "map":
            self.pages.setCurrentWidget(self.map_page)
        elif page == "effect":
            self.pages.setCurrentWidget(self.effect_page)
        elif page == "zones":
            self.pages.setCurrentWidget(self.zones_page)

    def create_general_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        self.set_page_title(layout, "General Settings")
        page.setLayout(layout)
        return page

    def create_map_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        self.set_page_title(layout, "Map")
        page.setLayout(layout)
        return page

    def create_effect_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        self.set_page_title(layout, "Trainings Effect")
        page.setLayout(layout)
        return page

    def create_zones_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        self.set_page_title(layout, "Heart Rate Zones")
        page.setLayout(layout)
        return page

    @staticmethod
    def set_page_title(layout, title: str) -> None:
        title_label = QLabel(_(title))
        title_label.setStyleSheet(
            """
                font-size: 16px;
                font-weight: bold;
            """
        )
        layout.addWidget(title_label)