from PyQt6.QtWidgets import QHBoxLayout, QVBoxLayout, QWidget

from ui.widget_activity_details import ActivityDetailsWidget


def page_segments(
    page_title, db_handler, activity_id, activity_type
):
    page = QWidget()
    layout = QVBoxLayout()
    layout.setContentsMargins(20, 0, 0, 0)
    layout.addLayout(page_title)

    segment_widget = ActivityDetailsWidget(db_handler, activity_id, activity_type)
    layout.addWidget(segment_widget)
    page.setLayout(layout)
    return page
