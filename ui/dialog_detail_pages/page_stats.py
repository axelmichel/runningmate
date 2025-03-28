from PyQt6.QtWidgets import QVBoxLayout, QWidget

from ui.widget_elevation_chart import ElevationChartWidget
from ui.widget_heart_rate_chart import HeartRateChartWidget


def page_stats(
    page_title, db_handler, activity_id, activity_type, file_path, image_path
):
    page = QWidget()
    layout = QVBoxLayout()
    layout.setContentsMargins(20, 0, 0, 0)
    layout.addLayout(page_title)

    elevation_chart_widget = ElevationChartWidget(
        file_path, image_path, db_handler, activity_id, activity_type
    )
    layout.addWidget(elevation_chart_widget)
    heart_rate_chart_widget = HeartRateChartWidget(
        file_path, image_path, db_handler, activity_id, activity_type
    )
    layout.addWidget(heart_rate_chart_widget)
    page.setLayout(layout)
    return page
