from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QAbstractItemView,
    QHBoxLayout,
    QHeaderView,
    QPushButton,
    QTableWidget,
    QTableWidgetItem,
    QWidget,
)

from processing.system_settings import SortOrder, ViewMode
from utils.translations import _  # Import translation function


class NumericTableWidgetItem(QTableWidgetItem):
    """Custom TableWidgetItem that ensures proper numeric sorting."""

    def __init__(self, value):
        super().__init__(str(value))
        try:
            self.numeric_value = float(value)
        except ValueError:
            self.numeric_value = value

    def __lt__(self, other):
        if isinstance(other, NumericTableWidgetItem):
            try:
                return self.numeric_value < other.numeric_value
            except TypeError:
                return str(self.numeric_value) < str(other.numeric_value)
        return super().__lt__(other)


HEADERS = {
    ViewMode.ALL: ["activity_type", "date_time", "title", "duration", "distance"],
    ViewMode.RUN: [
        "date_time",
        "title",
        "duration",
        "distance",
        "elevation_gain",
        "avg_speed",
        "avg_pace",
        "avg_heart_rate",
        "total_steps",
        "pause",
    ],
    ViewMode.WALK: [
        "date_time",
        "title",
        "duration",
        "distance",
        "elevation_gain",
        "avg_speed",
        "avg_pace",
        "avg_heart_rate",
        "total_steps",
    ],
    ViewMode.CYCLE: [
        "date_time",
        "title",
        "duration",
        "distance",
        "elevation_gain",
        "avg_speed",
        "avg_pace",
        "avg_heart_rate",
        "avg_power",
        "pause",
    ],
}


class TableBuilder:
    COLUMN_ALIGNMENTS = {
        "duration": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "pause": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "distance": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "elevation_gain": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "avg_speed": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "avg_pace": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "avg_power": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "avg_heart_rate": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "total_steps": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "date_time": Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter,
        "activity_type": Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter,
        "fastest_pace": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "slowest_pace": Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter,
        "title": Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter,
    }

    @staticmethod
    def setup_table(
        table_widget: QTableWidget, activity_type: ViewMode, data: list, parent=None
    ):
        if not data:
            table_widget.row_data = []
            table_widget.clear()
            table_widget.clearContents()
            table_widget.setRowCount(0)
            return

        headers = HEADERS.get(activity_type, [])
        translated_headers = [_((header)) for header in headers]

        table_widget.clear()
        table_widget.setSelectionBehavior(
            QAbstractItemView.SelectionBehavior.SelectRows
        )
        table_widget.setStyleSheet(
            "QTableWidget::item:selected { background-color: red; }"
        )

        table_widget.clearContents()
        table_widget.verticalHeader().hide()
        table_widget.setRowCount(len(data))
        table_widget.setColumnCount(len(headers) + 3)
        table_widget.setStyleSheet("QHeaderView::section { height: 30px; }")
        table_widget.setHorizontalHeaderLabels(
            translated_headers + ["_id", _("Actions"), ""]
        )
        table_widget.horizontalHeader().setSectionResizeMode(
            QHeaderView.ResizeMode.ResizeToContents
        )
        table_widget.verticalHeader().setDefaultSectionSize(40)
        table_widget.setAlternatingRowColors(True)
        table_widget.setShowGrid(True)
        table_widget.setGridStyle(Qt.PenStyle.SolidLine)
        table_widget.horizontalHeader().setStretchLastSection(True)
        table_widget.row_data = data

        for row_index, row_data in enumerate(data):
            for col_index, key in enumerate(headers):
                value = row_data.get(key, "")
                item = NumericTableWidgetItem(value)

                if key in TableBuilder.COLUMN_ALIGNMENTS:
                    item.setTextAlignment(TableBuilder.COLUMN_ALIGNMENTS[key])
                else:
                    item.setTextAlignment(
                        Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
                    )

                table_widget.setItem(row_index, col_index, item)

            # Add actions column
            actions_widget = TableBuilder.getActions(table_widget, row_index, parent)
            table_widget.setCellWidget(row_index, len(headers) + 1, actions_widget)

            empty_item = QTableWidgetItem("")
            empty_item.setFlags(
                Qt.ItemFlag.ItemIsEnabled
            )  # Optional: make it non-editable
            table_widget.setItem(row_index, len(headers) + 2, empty_item)

            id_item = QTableWidgetItem(
                str(row_data.get("activity_id", row_index))
            )  # Use row index if no ID
            id_item.setFlags(Qt.ItemFlag.ItemIsEnabled)  # Prevent editing
            table_widget.setItem(
                row_index, len(headers), id_item
            )  # Store ID in last column

        table_widget.setColumnHidden(len(headers), True)

        try:
            table_widget.cellClicked.disconnect()
            table_widget.horizontalHeader().sectionClicked.disconnect()
        except TypeError:
            pass  # If not connected before, ignore the error

        if parent:
            table_widget.cellClicked.connect(
                lambda row, col: TableBuilder.handle_row_click(
                    table_widget, row, parent
                )
            )
            table_widget.horizontalHeader().sectionClicked.connect(
                lambda index: parent.sort_by_column(
                    activity_type, column=headers[index]
                )
            )

    @staticmethod
    def update_header_styles(
        table_widget: QTableWidget,
        activity_type: ViewMode,
        column,
        sort_order: SortOrder,
    ):
        """
        Update the QTableWidget header to indicate the sorting column and direction.
        """
        header = table_widget.horizontalHeader()
        headers = HEADERS.get(activity_type, [])

        if column not in headers:
            return

        column_index = headers.index(column)
        order = (
            Qt.SortOrder.AscendingOrder
            if sort_order == SortOrder.ASC
            else Qt.SortOrder.DescendingOrder
        )
        header.setSortIndicator(column_index, order)
        header.setSortIndicatorShown(True)

    @staticmethod
    def getActions(table_widget, row_index, parent):
        # Actions Column (Buttons)
        actions_widget = QWidget()
        layout = QHBoxLayout()
        layout.setContentsMargins(
            0, 0, 0, 0
        )  # Remove margins for better button alignment

        # Refresh Button
        refresh_button = QPushButton(_("Refresh"))
        refresh_button.setStyleSheet("padding: 2px; font-size: 12px;")
        refresh_button.clicked.connect(
            lambda _, row=row_index: TableBuilder.handle_action_click(
                table_widget, row, parent, "refresh"
            )
        )

        # Delete Button
        delete_button = QPushButton(_("Delete"))
        delete_button.setStyleSheet("color: red; padding: 2px; font-size: 12px;")
        delete_button.clicked.connect(
            lambda _, row=row_index: TableBuilder.handle_action_click(
                table_widget, row, parent, "delete"
            )
        )

        layout.addWidget(refresh_button)
        layout.addWidget(delete_button)
        layout.addStretch()
        actions_widget.setLayout(layout)
        return actions_widget

    def handle_action_click(table_widget, row, parent, action):
        """Finds the correct row index after sorting and passes the correct row data."""
        id_item = table_widget.item(
            row, table_widget.columnCount() - 3
        )  # ✅ Get the hidden ID column
        if id_item:
            row_id = id_item.text()  # ✅ Extract the stored ID
            if action == "delete":
                parent.delete_entry(row_id)
            elif action == "refresh":
                parent.refresh_entry(row_id)

    @staticmethod
    def handle_row_click(table_widget, row, parent):
        """Finds the correct row index after sorting and passes the correct row data."""
        id_item = table_widget.item(
            row, table_widget.columnCount() - 3
        )  # ✅ Get the hidden ID column
        if id_item:
            row_id = id_item.text()  # ✅ Extract the stored ID
            correct_row_data = next(
                (
                    data
                    for data in table_widget.row_data
                    if str(data.get("activity_id")) == row_id
                ),
                None,
            )

            if correct_row_data:
                parent.load_detail(data=correct_row_data)  # ✅ Pass correct data
