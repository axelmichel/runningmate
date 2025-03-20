from PyQt6.QtWidgets import QVBoxLayout, QWidget

from ui.widget_trainings_effect import TrainingEffectWidget


def page_effect(page_title, db_handler, activity_id, activity_type, activity_date):
    page = QWidget()
    layout = QVBoxLayout()
    layout.setContentsMargins(20, 0, 0, 0)
    layout.addLayout(page_title)

    effect_widget = TrainingEffectWidget(
        db_handler, activity_id, activity_type, activity_date, 360
    )
    layout.addWidget(effect_widget, 5)
    layout.addStretch(5)
    page.setLayout(layout)
    return page
