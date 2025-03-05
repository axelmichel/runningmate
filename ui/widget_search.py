from PyQt6.QtCore import Qt, QDateTime
from PyQt6.QtWidgets import (
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from ui.dialog_search_filter import SearchFilterDialog
from ui.icon_button import IconButton
from utils.translations import _


class SearchWidget(QWidget):
    def __init__(self, db_handler: DatabaseHandler, search_callback=None, parent=None):
        super().__init__(parent)
        self.raw_filters = None
        self.filters = {}
        self.conn = db_handler.conn
        self.search_callback = search_callback

        main_layout = QVBoxLayout()
        main_layout.setContentsMargins(0, 0, 0, 0)

        search_row = QHBoxLayout()

        self.search_field = QLineEdit()
        self.search_field.setMinimumWidth(300)
        self.search_field.setStyleSheet("QLineEdit { padding: 5px 10px;}")
        self.search_field.setPlaceholderText(_("Search title or comment..."))
        self.search_field.textChanged.connect(self.update_reset_button_state)
        search_row.addWidget(self.search_field)

        self.filter_button = IconButton("filter-fill.svg")
        self.filter_button.clicked.connect(self.open_filter_dialog)
        search_row.addWidget(self.filter_button)
        search_row.addStretch(1)

        self.search_button = QPushButton(_("Search"))
        self.search_button.clicked.connect(self.search)
        search_row.addWidget(self.search_button)

        self.reset_button = QPushButton(_("Reset"))
        self.reset_button.setEnabled(False)  # Initially disabled
        self.reset_button.clicked.connect(self.reset_search)
        search_row.addWidget(self.reset_button)

        main_layout.addLayout(search_row)

        self.filter_summary_label = QLabel("")
        self.filter_summary_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        main_layout.addWidget(self.filter_summary_label)

        self.setLayout(main_layout)

    def set_search_filters(self, filters):
        flattened = {}

        for filter_name, filter_values in filters.items():
            if "min" in filter_values:
                flattened[f"min_{filter_name}"] = filter_values["min"]
            if "max" in filter_values:
                flattened[f"max_{filter_name}"] = filter_values["max"]

        self.raw_filters = filters
        self.filters = flattened
        self.update_filter_summary()
        self.update_reset_button_state()

    def update_filter_summary(self):
        """Updates the filter summary text below the search bar with formatted values."""
        if not self.filters:
            self.filter_summary_label.setText("")
            return

        filter_texts = []

        for key, value in self.filters.items():
            prefix, filter_name = key.split("_", 1)

            if filter_name == "date":
                formatted_value = QDateTime.fromSecsSinceEpoch(value).toString("yyyy-MM-dd")

            elif filter_name == "duration":
                hours = value // 3600
                minutes = (value % 3600) // 60
                seconds = value % 60
                formatted_value = f"{hours:02d}:{minutes:02d}:{seconds:02d}"

            elif filter_name == "distance":
                formatted_value = f"{value} km"

            elif filter_name == "elevation":
                formatted_value = f"{value} m"

            else:
                formatted_value = str(value)

            filter_texts.append(f"{filter_name.capitalize()} ({prefix}): {formatted_value}")

        self.filter_summary_label.setText(", ".join(filter_texts))

    def update_reset_button_state(self):
        """Enables reset button only if search field or filters are not empty."""
        has_text = bool(self.search_field.text().strip())
        has_filters = bool(self.filters)
        self.reset_button.setEnabled(has_text or has_filters)

    def reset_search(self):
        """Clears search field, filters, and updates UI."""
        self.search_field.clear()
        self.filters = {}
        self.raw_filters = None
        self.filter_summary_label.setText("")
        self.update_reset_button_state()
        if self.search_callback:
            self.search_callback({})  #

    def open_filter_dialog(self):
        dialog = SearchFilterDialog(self)
        if self.raw_filters:
            dialog.set_filters(self.raw_filters)
        dialog.exec()

    def search(self):
        if self.search_callback:
            self.search_callback(self.filters)
