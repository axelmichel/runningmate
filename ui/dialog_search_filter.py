from PyQt6.QtCore import QDate, QDateTime, Qt, QTime
from PyQt6.QtWidgets import (
    QDateEdit,
    QDialog,
    QDoubleSpinBox,
    QHBoxLayout,
    QLabel,
    QSpinBox,
    QVBoxLayout,
)

from ui.dialog_action_bar import DialogActionBar
from utils.translations import _


class SearchFilterDialog(QDialog):
    """Dialog to filter activities based on Date, Distance, Duration, and Elevation."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle(_("Search Filters"))
        self.setGeometry(400, 200, 400, 250)

        layout = QVBoxLayout()

        # ✅ Row 1: Date Range (QDateEdit)
        self.date_from = QDateEdit()
        self.date_from.setCalendarPopup(True)
        self.date_from.setDate(QDate.currentDate())

        self.date_to = QDateEdit()
        self.date_to.setCalendarPopup(True)
        self.date_to.setDate(QDate.currentDate())

        # 🔹 Ensure validation only happens when modifying max date
        self.date_to.dateChanged.connect(self.validate_dates)

        layout.addLayout(self.create_row(_("Date"), self.date_from, self.date_to))

        # ✅ Distance Filter
        self.distance_from = QDoubleSpinBox()
        self.distance_from.setRange(0, 1000)
        self.distance_from.setSuffix(" km")

        self.distance_to = QDoubleSpinBox()
        self.distance_to.setRange(0, 1000)
        self.distance_to.setSuffix(" km")

        # 🔹 Ensure validation only happens when modifying max distance
        self.distance_to.valueChanged.connect(self.validate_distances)

        layout.addLayout(self.create_row(_("Distance"), self.distance_from, self.distance_to))

        # ✅ Duration Filter
        self.duration_from_h = self.create_spinbox(0, 23)
        self.duration_from_m = self.create_spinbox(0, 59)
        self.duration_from_s = self.create_spinbox(0, 59)

        self.duration_to_h = self.create_spinbox(0, 23)
        self.duration_to_m = self.create_spinbox(0, 59)
        self.duration_to_s = self.create_spinbox(0, 59)

        duration_layout = QHBoxLayout()
        duration_layout.addWidget(QLabel("From:"))
        duration_layout.addWidget(self.duration_from_h)
        duration_layout.addWidget(QLabel(":"))
        duration_layout.addWidget(self.duration_from_m)
        duration_layout.addWidget(QLabel(":"))
        duration_layout.addWidget(self.duration_from_s)

        duration_layout.addWidget(QLabel(" To:"))
        duration_layout.addWidget(self.duration_to_h)
        duration_layout.addWidget(QLabel(":"))
        duration_layout.addWidget(self.duration_to_m)
        duration_layout.addWidget(QLabel(":"))
        duration_layout.addWidget(self.duration_to_s)

        # 🔹 Ensure validation only happens when modifying max duration
        self.duration_to_h.valueChanged.connect(self.validate_durations)
        self.duration_to_m.valueChanged.connect(self.validate_durations)
        self.duration_to_s.valueChanged.connect(self.validate_durations)

        layout.addWidget(QLabel(_("Duration")))
        layout.addLayout(duration_layout)

        # ✅ Elevation Filter
        self.elevation_from = QSpinBox()
        self.elevation_from.setRange(0, 10000)

        self.elevation_to = QSpinBox()
        self.elevation_to.setRange(0, 10000)

        # 🔹 Ensure validation only happens when modifying max elevation
        self.elevation_to.valueChanged.connect(self.validate_elevations)

        layout.addLayout(self.create_row(_("Elevation"), self.elevation_from, self.elevation_to))

        self.action_bar = DialogActionBar(
            cancel_action=self.close,
            submit_action=self.apply_filters,
            submit_label=_("Set Filters"),
        )
        layout.addWidget(self.action_bar)
        self.setLayout(layout)

    @staticmethod
    def create_row(title, from_widget, to_widget):
        """Helper function to create a row layout with title and two input fields."""
        row_layout = QHBoxLayout()
        row_layout.addWidget(QLabel(title))
        row_layout.addWidget(from_widget)
        row_layout.addWidget(QLabel(" - "))
        row_layout.addWidget(to_widget)
        return row_layout

    @staticmethod
    def create_spinbox(min_val, max_val):
        """Helper function to create a QSpinBox for hours, minutes, seconds."""
        spinbox = QSpinBox()
        spinbox.setRange(min_val, max_val)
        spinbox.setFixedWidth(50)
        spinbox.setAlignment(Qt.AlignmentFlag.AlignCenter)
        return spinbox

    def set_filters(self, filters):
        """Set initial values for the filter fields."""
        date_filter = filters.get("date", {})

        if "min" in date_filter and date_filter["min"] > 0:
            self.date_from.setDate(QDateTime.fromSecsSinceEpoch(date_filter["min"]).date())
        else:
            self.date_from.setDate(QDate.currentDate())  # Default to today

        if "max" in date_filter and date_filter["max"] > 0:
            self.date_to.setDate(QDateTime.fromSecsSinceEpoch(date_filter["max"]).date())
        else:
            self.date_to.setDate(QDate.currentDate())  # Default to today

        distance_filter = filters.get("distance", {})
        self.distance_from.setValue(distance_filter.get("min", 0))
        self.distance_to.setValue(distance_filter.get("max", 0))

        duration_filter = filters.get("duration", {})
        min_duration = duration_filter.get("min", 0)
        max_duration = duration_filter.get("max", 0)

        self.duration_from_h.setValue(min_duration // 3600)  # Hours
        self.duration_from_m.setValue((min_duration % 3600) // 60)  # Minutes
        self.duration_from_s.setValue(min_duration % 60)  # Seconds

        self.duration_to_h.setValue(max_duration // 3600)  # Hours
        self.duration_to_m.setValue((max_duration % 3600) // 60)  # Minutes
        self.duration_to_s.setValue(max_duration % 60)  # Seconds

        elevation_filter = filters.get("elevation", {})
        self.elevation_from.setValue(elevation_filter.get("min", 0))
        self.elevation_to.setValue(elevation_filter.get("max", 0))

    def validate_dates(self):
        """Ensures that max date is greater than or equal to the min date, but only when max is modified."""
        if self.date_to.date() < self.date_from.date():
            self.date_to.setDate(self.date_from.date())

    def validate_distances(self):
        """Ensures that max distance is greater than or equal to min distance, but only when max is modified."""
        if self.distance_to.value() < self.distance_from.value():
            self.distance_to.setValue(self.distance_from.value())

    def validate_durations(self):
        """Ensures that max duration is greater than or equal to min duration, but only when max is modified."""
        min_duration = (
            self.duration_from_h.value() * 3600 +
            self.duration_from_m.value() * 60 +
            self.duration_from_s.value()
        )
        max_duration = (
            self.duration_to_h.value() * 3600 +
            self.duration_to_m.value() * 60 +
            self.duration_to_s.value()
        )
        if max_duration < min_duration:
            self.duration_to_h.setValue(self.duration_from_h.value())
            self.duration_to_m.setValue(self.duration_from_m.value())
            self.duration_to_s.setValue(self.duration_from_s.value())

    def validate_elevations(self):
        """Ensures that max elevation is greater than or equal to min elevation, but only when max is modified."""
        if self.elevation_to.value() < self.elevation_from.value():
            self.elevation_to.setValue(self.elevation_from.value())

    def apply_filters(self):  # noqa: C901
        """Collects data from fields and triggers the parent function."""
        filters = {}

        # Convert Duration filters into total seconds
        from_duration_sec = (
                self.duration_from_h.value() * 3600 +
                self.duration_from_m.value() * 60 +
                self.duration_from_s.value()
        )
        to_duration_sec = (
                self.duration_to_h.value() * 3600 +
                self.duration_to_m.value() * 60 +
                self.duration_to_s.value()
        )

        if self.date_from.date() != QDate.currentDate() and self.date_to.date() != QDate.currentDate():
            self.validate_dates()
        if self.distance_from.value() > 0 and self.distance_to.value() > 0:
            self.validate_distances()
        if from_duration_sec > 0 and to_duration_sec > 0:
            self.validate_durations()
        if self.elevation_from.value() > 0 and self.elevation_to.value() > 0:
            self.validate_elevations()

        # Convert Date filters to Unix timestamp (only include if explicitly set)
        if self.date_from.date() != QDate.currentDate():
            date_min = QDateTime(self.date_from.date(), QTime(0, 0)).toSecsSinceEpoch()
            filters.setdefault("date", {})["min"] = date_min

        if self.date_to.date() != QDate.currentDate():
            date_max = QDateTime(self.date_to.date(), QTime(23, 59, 59)).toSecsSinceEpoch()
            filters.setdefault("date", {})["max"] = date_max

        # Distance filter: Only include min/max if > 0
        if self.distance_from.value() > 0:
            filters.setdefault("distance", {})["min"] = self.distance_from.value()
        if self.distance_to.value() > 0:
            filters.setdefault("distance", {})["max"] = self.distance_to.value()

        if from_duration_sec > 0:
            filters.setdefault("duration", {})["min"] = from_duration_sec
        if to_duration_sec > 0:
            filters.setdefault("duration", {})["max"] = to_duration_sec

        # Elevation filter: Only include min/max if > 0
        if self.elevation_from.value() > 0:
            filters.setdefault("elevation", {})["min"] = self.elevation_from.value()
        if self.elevation_to.value() > 0:
            filters.setdefault("elevation", {})["max"] = self.elevation_to.value()

        # Pass filters to parent if applicable
        if self.parent() and hasattr(self.parent(), "set_search_filters"):
            self.parent().set_search_filters(filters)

        self.accept()