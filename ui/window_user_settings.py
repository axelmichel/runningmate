from PyQt6.QtCore import Qt
from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import (
    QCheckBox,
    QDialog,
    QFormLayout,
    QFrame,
    QHBoxLayout,
    QHeaderView,
    QLabel,
    QLineEdit,
    QPushButton,
    QStackedWidget,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)

from database.user_settings import UserSettings
from ui.opacity_button import OpacityButton, get_opacity_button_style
from ui.themes import THEME
from utils.app_mode import is_dark_mode
from utils.resource_path import resource_path
from utils.translations import _


class UserSettingsWindow(QDialog):

    def __init__(self, user_settings: UserSettings , parent=None):
        super().__init__()

        self.pages = None
        self.general_page = None
        self.shoes_page = None
        self.bikes_page = None
        self.heart_rate_page = None

        self.btn_heart_rate = None
        self.btn_bikes = None
        self.btn_shoes = None
        self.btn_general = None

        self.height_input = None
        self.birthday_input = None
        self.weight_input = None
        self.name_input = None
        self.hr_max_input = None
        self.hr_min_input = None

        self.vo2max_input = None
        self.zone1_input = None
        self.zone2_input = None
        self.zone3_input = None
        self.zone4_input = None
        self.zone5_input = None

        self.bike_status = None
        self.bike_name = None
        self.bike_weight = None
        self.bikes_table = None

        self.shoe_status = None
        self.shoe_name = None
        self.shoes_table = None

        self.icon_folder = "light" if is_dark_mode() else "dark"
        self.db = user_settings
        self.parent = parent
        self.user = self.db.get_user_data()
        self.init_ui()

    def init_ui(self) -> None:
        """
        Initializes the UI components and menu navigation with a real vertical sidebar.
        """
        self.setWindowTitle("User Settings")
        self.setGeometry(100, 100, 800, 500)

        main_layout = QHBoxLayout()
        sidebar_layout = QVBoxLayout()
        sidebar_layout.setAlignment(Qt.AlignmentFlag.AlignTop)
        sidebar_frame = QFrame()
        sidebar_frame.setLayout(sidebar_layout)
        sidebar_frame.setFixedWidth(200)

        self.pages = QStackedWidget()

        self.general_page = self.create_general_page()
        self.shoes_page = self.create_shoes_page()
        self.bikes_page = self.create_bikes_page()
        self.heart_rate_page = self.create_heart_rate_page()

        self.pages.addWidget(self.general_page)
        self.pages.addWidget(self.shoes_page)
        self.pages.addWidget(self.bikes_page)
        self.pages.addWidget(self.heart_rate_page)

        def add_button(icon, tooltip, page):
            button = OpacityButton(f"  {_(tooltip)}")
            button.setCheckable(True)
            button.setAutoExclusive(True)
            button.setStyleSheet(get_opacity_button_style())
            button.setIcon(QIcon(resource_path(f"icons/{self.icon_folder}/{icon}.svg")))
            button.setToolTip(_(tooltip))
            button.clicked.connect(lambda: self.set_active_page(page, button))
            sidebar_layout.addWidget(button)
            return button

        self.btn_general = add_button("user-line", "General", self.general_page)
        self.btn_shoes = add_button("footprint-fill", "Shoes", self.shoes_page)
        self.btn_bikes = add_button("bike-line", "Bikes", self.bikes_page)
        self.btn_heart_rate = add_button("heart-pulse-line", "Heart Rate Zones", self.heart_rate_page)

        self.update_menu_buttons()

        main_layout.addWidget(sidebar_frame)
        main_layout.addWidget(self.pages, 1)
        self.setLayout(main_layout)
        self.btn_general.setChecked(True)

    def set_active_page(self, page, button=None) -> None:
        if button:
            button.setChecked(True)
        self.pages.setCurrentWidget(page)

    def update_menu_buttons(self) -> None:
        """
        Updates the state of menu buttons. If no user is present, disables them.
        """
        user_available = self.user is not None
        self.btn_shoes.setEnabled(user_available)
        self.btn_bikes.setEnabled(user_available)
        self.btn_heart_rate.setEnabled(user_available)

    def create_general_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout()
        self.set_page_title(layout, "General Settings")

        form_layout = self.get_form()

        self.name_input = self.get_form_field()
        self.weight_input = self.get_form_field()
        self.birthday_input = self.get_form_field()
        self.height_input = self.get_form_field()
        self.hr_min_input = self.get_form_field()
        self.hr_max_input = self.get_form_field()

        form_layout.addRow(_("Name:"), self.name_input)
        form_layout.addRow(_("Weight (kg):"), self.weight_input)
        form_layout.addRow(_("Birthday (DD.MM.YYYY):"), self.birthday_input)
        form_layout.addRow(_("Height (cm):"), self.height_input)
        form_layout.addRow(_("HRmax:"), self.hr_max_input)
        form_layout.addRow(_("HRmin:"), self.hr_min_input)
        save_btn = QPushButton(_("Save"))
        save_btn.clicked.connect(self.save_general_settings)
        layout.addLayout(form_layout)
        self.load_general_settings()
        layout.addWidget(save_btn)
        layout.addStretch(1)

        page.setLayout(layout)
        return page

    def get_form_field(self):
        field = QLineEdit()
        field.setMinimumWidth(200)
        field.setStyleSheet("QLineEdit { padding: 5px 10px;}")
        return field

    def get_form(self) -> QFormLayout:
        form_layout = QFormLayout()
        form_layout.setAlignment(Qt.AlignmentFlag.AlignLeft)
        form_layout.setSpacing(20)
        form_layout.setFieldGrowthPolicy(QFormLayout.FieldGrowthPolicy.ExpandingFieldsGrow)
        form_layout.setFormAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
        form_layout.setContentsMargins(0, 10, 0, 10)
        return form_layout

    def load_general_settings(self) -> None:
        if self.user:
            if self.user["name"]:
                self.name_input.setText(self.user["name"])
            if self.user["weight"]:
                self.weight_input.setText(str(self.user["weight"]))
            if self.user["birthday"]:
                self.birthday_input.setText(str(self.user["birthday"]))
            if self.user["height"]:
                self.height_input.setText(str(self.user["height"]))
            if self.user["hr_min"]:
                self.hr_min_input.setText(str(self.user["hr_min"]))
            if self.user["hr_max"]:
                self.hr_max_input.setText(str(self.user["hr_max"]))
        else:
            self.hr_max_input.setText("220")
            self.hr_min_input.setText("110")

    def save_general_settings(self) -> None:
        id = self.user["id"] if self.user else None
        name = self.name_input.text()
        weight = float(self.weight_input.text())
        birthday = self.birthday_input.text()
        height = int(self.height_input.text())
        hr_min = int(self.hr_min_input.text())
        hr_max = int(self.hr_max_input.text())
        self.db.insert_or_update_user(name, weight, height, hr_min, hr_max, birthday, id)
        self.user = self.db.get_user_data()
        self.update_menu_buttons()

    def create_shoes_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout()
        self.set_page_title(layout, "Shoes")

        self.shoes_table = self.get_table()
        self.shoes_table.setColumnCount(4)
        self.shoes_table.setHorizontalHeaderLabels([_("Shoe"), _("Distance"), _("In Use"), ""])
        self.shoes_table.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)
        self.load_shoes()
        table_layout = QVBoxLayout()
        table_layout.addWidget(QLabel(_("Existing Shoes:")))
        table_layout.setSpacing(10)
        table_layout.addWidget(self.shoes_table)
        layout.addLayout(table_layout)


        form_layout = self.get_form()
        self.shoe_name = self.get_form_field()
        self.shoe_status = QCheckBox("In Use")
        self.shoe_status.setChecked(True)
        form_layout.addRow("Name:", self.shoe_name)
        form_layout.addRow("Status:", self.shoe_status)
        save_btn = QPushButton(_("Add"))
        save_btn.clicked.connect(self.save_shoe)

        form_box = QVBoxLayout()
        form_box.setSpacing(10)
        title_label = QLabel(_('Add Shoe'))
        title_label.setStyleSheet(
            """
                font-size: 14px;
                font-weight: bold;
            """
        )
        form_box.addWidget(title_label)
        form_box.addLayout(form_layout)
        form_box.addWidget(save_btn)
        layout.addLayout(form_box)

        page.setLayout(layout)
        return page

    def set_page_title(self, layout, title: str) -> None:
        title_label = QLabel(_(title))
        title_label.setStyleSheet(
            """
                font-size: 16px;
                font-weight: bold;
            """
        )
        layout.addWidget(title_label)

    def save_shoe(self) -> None:
        name = self.shoe_name.text()
        status = self.shoe_status.isChecked()
        self.db.insert_shoe(name, status)
        self.load_shoes()

    def create_bikes_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout()

        self.set_page_title(layout, "Bikes")

        self.bikes_table = self.get_table()
        self.bikes_table.setColumnCount(5)
        self.bikes_table.setHorizontalHeaderLabels([_("Bike"), _("Weight"), _("Distance"), _("In Use"), ""])
        self.bikes_table.horizontalHeader().setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)
        self.load_bikes()

        table_layout = QVBoxLayout()
        table_layout.setSpacing(10)
        table_layout.addWidget(self.bikes_table)

        layout.addLayout(table_layout)

        form_layout = self.get_form()
        self.bike_name = self.get_form_field()
        self.bike_weight = self.get_form_field()
        self.bike_status = QCheckBox("In Use")
        self.bike_status.setChecked(True)
        form_layout.addRow("Name:", self.bike_name)
        form_layout.addRow("Weight (KG):", self.bike_weight)
        form_layout.addRow("Status:", self.bike_status)
        save_btn = QPushButton(_("Add"))
        save_btn.clicked.connect(self.save_bike)

        form_box = QVBoxLayout()
        form_box.setSpacing(10)
        title_label = QLabel(_('Add Bike'))
        title_label.setStyleSheet(
            """
                font-size: 14px;
                font-weight: bold;
            """
        )
        form_box.addWidget(title_label)
        form_box.addLayout(form_layout)
        form_box.addWidget(save_btn)
        layout.addLayout(form_box)

        page.setLayout(layout)

        return page

    def save_bike(self) -> None:
        name = self.bike_name.text()
        status = self.bike_status.isChecked()
        weight = float(self.bike_weight.text())
        self.db.insert_bike(name, weight, status)
        self.load_bikes()

    def create_heart_rate_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout()

        self.set_page_title(layout, "Heart Rate Zones")

        form_layout = self.get_form()
        self.vo2max_input = self.get_form_field()
        self.zone1_input = self.get_form_field()
        self.zone2_input = self.get_form_field()
        self.zone3_input = self.get_form_field()
        self.zone4_input = self.get_form_field()
        self.zone5_input = self.get_form_field()

        form_layout.addRow("VO2max:", self.vo2max_input)
        form_layout.addRow("Zone 1 MAX:", self.zone1_input)
        form_layout.addRow("Zone 2 MAX:", self.zone2_input)
        form_layout.addRow("Zone 3 MAX:", self.zone3_input)
        form_layout.addRow("Zone 4 MAX:", self.zone4_input)
        form_layout.addRow("Zone 5 MAX:", self.zone5_input)

        save_btn = QPushButton(_("Save Heart Rate Zones"))
        save_btn.clicked.connect(self.save_heart_rate_zones)
        layout.addLayout(form_layout)
        layout.addWidget(save_btn)
        layout.addStretch(1)

        self.load_heart_rate_data()
        page.setLayout(layout)

        return page

    def load_heart_rate_data(self) -> None:
        """
        Loads existing heart rate zone values from the database.
        If no values are present, it calculates default values based on user data.
        """
        if self.user:
            if self.user["vo2max"]:
                self.vo2max_input.setText(str( self.user["vo2max"]))
            if self.user["zone1"]:
                self.zone1_input.setText(str( self.user["zone1"]))
            if self.user["zone2"]:
                self.zone2_input.setText(str( self.user["zone2"]))
            if self.user["zone3"]:
                self.zone3_input.setText(str( self.user["zone3"]))
            if self.user["zone4"]:
                self.zone4_input.setText(str( self.user["zone4"]))
            if self.user["zone5"]:
                self.zone5_input.setText(str(self.user["zone5"]))
            if not any([self.user["zone1"], self.user["zone2"], self.user["zone3"], self.user["zone4"], self.user["zone5"]]) and self.user["age"]:
                self.calculate_heart_rate_zones()

    def calculate_heart_rate_zones(self) -> None:
        """
        Calculates default heart rate zones based on age using a standard formula.
        """
        hr_max = self.user["hr_max"] if self.user["hr_max"] else 220
        max_hr = hr_max - self.user["age"]
        self.zone1_input.setText(str(int(max_hr * 0.6)))
        self.zone2_input.setText(str(int(max_hr * 0.7)))
        self.zone3_input.setText(str(int(max_hr * 0.8)))
        self.zone4_input.setText(str(int(max_hr * 0.9)))
        self.zone5_input.setText(str(max_hr))

    def save_heart_rate_zones(self) -> None:
        """
        Saves the heart rate zones and VO2max into the database.
        """
        if not self.user["id"]:
            return
        vo2max = float(self.vo2max_input.text() or 0)
        hr_min = int(self.hr_min_input.text() or 0)
        zone1 = int(self.zone1_input.text() or 0)
        zone2 = int(self.zone2_input.text() or 0)
        zone3 = int(self.zone3_input.text() or 0)
        zone4 = int(self.zone4_input.text() or 0)
        zone5 = int(self.zone5_input.text() or 0)
        self.db.set_heart_rates_zones(self.user["id"], vo2max, hr_min, zone1, zone2, zone3, zone4, zone5)

    def load_shoes(self) -> None:
        shoes = self.db.get_shoes()
        self.shoes_table.setRowCount(len(shoes))
        for row_index, row_data in enumerate(shoes):
            name = row_data.get("name", f"{_("shoe")} {row_index + 1}")
            distance = str(row_data.get("distance", "0.0"))
            status = row_data.get("status", False)

            name_item = QTableWidgetItem(name)
            name_item.setTextAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
            self.shoes_table.setItem(row_index, 0, name_item)

            distance_item = QTableWidgetItem(distance)
            distance_item.setTextAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
            self.shoes_table.setItem(row_index, 1, distance_item)

            status_checkbox = QCheckBox()
            status_checkbox.setChecked(status)
            status_checkbox.stateChanged.connect(lambda state, id=row_data["id"]: self.db.update_shoe_status(id, bool(state)))
            self.shoes_table.setCellWidget(row_index, 2, status_checkbox)

            delete_button = self.get_delete_button()
            delete_button.clicked.connect(lambda _, id=row_data["id"]: self.db.delete_shoe(id))
            self.shoes_table.setCellWidget(row_index, 3, delete_button)

    def load_bikes(self) -> None:
        bikes = self.db.get_bikes()
        self.bikes_table.setRowCount(len(bikes))
        for row_index, row_data in enumerate(bikes):
            name = row_data.get("name", f"{_("bike")} {row_index + 1}")
            distance = str(row_data.get("distance", "0.0"))
            status = row_data.get("status", False)
            weight = str(row_data.get("weight", "0.0"))

            name_item = QTableWidgetItem(name)
            name_item.setTextAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
            self.bikes_table.setItem(row_index, 0, name_item)

            weight_item = QTableWidgetItem(weight)
            weight_item.setTextAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
            self.bikes_table.setItem(row_index, 1, weight_item)

            distance_item = QTableWidgetItem(distance)
            distance_item.setTextAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
            self.bikes_table.setItem(row_index, 2, distance_item)

            # Status Checkbox
            status_checkbox = QCheckBox()
            status_checkbox.setChecked(status)
            status_checkbox.stateChanged.connect(
                lambda state, bike_id=row_data["id"]: self.db.update_bike_status(bike_id, bool(state)))
            self.bikes_table.setCellWidget(row_index, 3, status_checkbox)

            # Delete Button
            delete_button = self.get_delete_button()
            delete_button.clicked.connect(lambda _, bike_id=row_data["id"]: self.db.delete_bike(bike_id))
            self.bikes_table.setCellWidget(row_index, 4, delete_button)

    @staticmethod
    def get_table():
        table_widget = QTableWidget()
        table_widget.verticalHeader().setDefaultSectionSize(40)
        table_widget.setAlternatingRowColors(True)
        table_widget.setShowGrid(True)
        table_widget.setGridStyle(Qt.PenStyle.SolidLine)
        table_widget.horizontalHeader().setStretchLastSection(True)
        table_widget.setStyleSheet(
            """
            QTableWidget::item:selected {
                background-color: #333333;
                color: white;
            }
            QTableWidget::item {
                selection-background-color: transparent;
                selection-color: inherit;
                padding: 5px;
            }
        """
        )
        return table_widget

    def get_delete_button(self):
        delete_button = QPushButton()
        delete_button.setIcon(QIcon(resource_path(f"icons/{self.icon_folder}/close-circle-line.svg")))
        delete_button.setToolTip(_("Delete"))
        delete_button.setFixedSize(30, 30)
        delete_button.setStyleSheet(
            f"""
               QPushButton {{
                   background-color: {THEME.DELETE_COLOR};
                   border-radius: 5px;
                   padding: 5px;
               }}
               QPushButton:hover {{
                   background-color: {THEME.DELETE_HOVER};
               }}
           """)
        return delete_button