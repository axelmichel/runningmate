from PyQt6.QtWidgets import QVBoxLayout, QWidget

from ui.widget_map import MapWidget


def page_map(page_title, db_handler, activity_id, file_path, image_path):
    page = QWidget()
    layout = QVBoxLayout()
    layout.setContentsMargins(20, 0, 0, 0)
    layout.addLayout(page_title)

    track_map_widget = MapWidget(
        file_path, image_path, db_handler, activity_id, "track"
    )
    layout.addWidget(track_map_widget)
    page.setLayout(layout)
    return page
