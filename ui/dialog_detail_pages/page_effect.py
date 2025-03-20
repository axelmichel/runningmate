from PyQt6.QtWidgets import QVBoxLayout, QWidget

from ui.widget_fitness import FitnessFatigueWidget
from ui.widget_trainings_effect import TrainingEffectWidget


def page_effect(page_title, db_handler, activity_id, activity_type, activity_date):
    page = QWidget()
    layout = QVBoxLayout()
    layout.setContentsMargins(20, 0, 0, 0)
    layout.addLayout(page_title)

    effect_widget = TrainingEffectWidget(
        db_handler, activity_id, activity_type, activity_date, 360
    )
    layout.addWidget(effect_widget, 3)
    layout.addStretch(1)
    fitness_widget = FitnessFatigueWidget(db_handler, activity_date)
    layout.addWidget(fitness_widget, 6)
    page.setLayout(layout)
    return page
