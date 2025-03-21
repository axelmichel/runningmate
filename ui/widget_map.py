from PyQt6.QtCore import Qt, QUrl
from PyQt6.QtWebEngineCore import QWebEngineSettings
from PyQt6.QtWebEngineWidgets import QWebEngineView
from PyQt6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QSizePolicy,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from processing.track_map import TrackMap
from ui.icon_button import IconButton
from utils.translations import _


class MapWidget(QWidget):
    def __init__(
        self,
        file_path: str,
        image_path: str,
        db_handler: DatabaseHandler,
        activity_id: int,
        map_type: str,
    ):
        super().__init__()
        self.activity_id = activity_id
        self.TrackMap = TrackMap(file_path, image_path, db_handler, self.activity_id)
        self.map_type = map_type
        self.title_label = None
        self.web_view = QWebEngineView()
        self._init_ui()

    def _init_ui(self):
        track_map = self.TrackMap.create_map(self.map_type)
        if not track_map:
            return
        layout = QVBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)

        # Title Label
        self.title_label = QLabel(_(f"{self.map_type.capitalize()} Map"))
        self.title_label.setStyleSheet(
            """
            font-size: 13px;
            font-weight: bold;
            """
        )
        self.title_label.setAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
        )

        # Button Layout
        self.map_buttons_layout = QHBoxLayout()
        self.track_button = IconButton("map-pin-fill.svg")
        self.pace_button = IconButton("speed-up-line.svg")
        self.heart_rate_button = IconButton("heart-pulse-line.svg")

        self.track_button.clicked.connect(lambda: self._update_map("track"))
        self.pace_button.clicked.connect(lambda: self._update_map("pace"))
        self.heart_rate_button.clicked.connect(lambda: self._update_map("heart_rate"))

        self.map_buttons_layout.addWidget(self.track_button)
        self.map_buttons_layout.addWidget(self.pace_button)
        self.map_buttons_layout.addWidget(self.heart_rate_button)
        self.map_buttons_layout.setAlignment(
            Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter
        )

        # Title + Buttons in one row
        title_layout = QHBoxLayout()
        title_layout.addWidget(self.title_label)
        title_layout.addStretch()
        title_layout.addLayout(self.map_buttons_layout)

        layout.addLayout(title_layout)

        self.web_view.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        settings = self.web_view.settings()
        settings.setAttribute(QWebEngineSettings.WebAttribute.JavascriptEnabled, True)
        settings.setAttribute(
            QWebEngineSettings.WebAttribute.LocalContentCanAccessRemoteUrls, True
        )
        layout.addWidget(self.web_view)
        self.setLayout(layout)
        self._load_map(track_map)

    def _load_map(self, track_map=None):
        """Loads the map based on the map type."""
        if not track_map:
            track_map = self.TrackMap.create_map(self.map_type)
        self.web_view.setUrl(QUrl.fromLocalFile(track_map["file_path"]))

    def _update_map(self, new_map_type):
        """Updates the map when a button is clicked."""
        self.map_type = new_map_type
        self.title_label.setText(_(f"{self.map_type.capitalize()} Map"))
        self._load_map()
