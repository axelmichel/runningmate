from PyQt6.QtCore import Qt
from PyQt6.QtWebEngineWidgets import QWebEngineView
from PyQt6.QtWidgets import (
    QLabel,
    QSizePolicy,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from processing.chart_heart_rate import HeartRateChart
from processing.system_settings import ViewMode
from utils.translations import _


class HeartRateChartWidget(QWidget):
    def __init__(
        self,
        file_path: str,
        image_path: str,
        db_handler: DatabaseHandler,
        activity_id: int,
        activity_type=ViewMode.RUN,
    ):
        super().__init__()
        self.activity_id = activity_id
        self.chart = HeartRateChart(file_path, image_path, db_handler, activity_id, activity_type)
        self.web_view = QWebEngineView()
        self._init_ui()

    def _init_ui(self):
        chart_html = self.chart.create_chart()
        if not chart_html:
            return
        layout = QVBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)
        title_label = QLabel(_("Heart rate over distance"))
        title_label.setStyleSheet(
            """
            font-size: 13px;
            font-weight: bold;
            """
        )
        title_label.setAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
        )

        layout.addWidget(title_label)
        self.web_view.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )

        layout.addWidget(self.web_view)
        self.web_view.setHtml(chart_html)
        self.setLayout(layout)


