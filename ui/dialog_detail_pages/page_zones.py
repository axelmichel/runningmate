from PyQt6.QtWidgets import QHBoxLayout, QVBoxLayout, QWidget

from ui.widget_heart_rate_tendency import HeartRateTrendWidget
from ui.widget_heart_rate_zones import HeartRateZoneWidget


def page_zones(
    page_title, db_handler, user_id, activity_id, activity_type, activity_date
):
    page = QWidget()
    layout = QVBoxLayout()
    layout.setContentsMargins(20, 0, 0, 0)
    layout.addLayout(page_title)

    row_layout = QHBoxLayout()
    row_layout.setContentsMargins(0, 0, 0, 20)

    zones_widget = HeartRateZoneWidget(db_handler, user_id, activity_id)
    row_layout.addWidget(zones_widget)
    row_layout.addStretch(1)

    layout.addLayout(row_layout)

    layout.addStretch(1)

    tendency_row = QHBoxLayout()
    tendency_widget = HeartRateTrendWidget(
        db_handler, user_id, activity_type, activity_date
    )
    tendency_row.addWidget(tendency_widget)
    tendency_row.setContentsMargins(0, 0, 0, 0)
    layout.addLayout(tendency_row)

    page.setLayout(layout)
    return page
