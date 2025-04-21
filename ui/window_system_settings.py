import os

from PyQt6.QtCore import Qt
from PyQt6.QtWidgets import (
    QComboBox,
    QDialog,
    QFormLayout,
    QFrame,
    QHBoxLayout,
    QLabel,
    QSplitter,
    QStackedWidget,
    QVBoxLayout,
    QWidget,
)

from processing.system_settings import load_settings_config, save_settings
from ui.dialog_action_bar import DialogActionBar
from ui.side_bar import Sidebar
from ui.themes import THEME
from utils.app_mode import is_dark_mode
from utils.folder_size import folder_size
from utils.media_stats import media_stats
from utils.translations import _


class SystemSettingsWindow(QDialog):

    def __init__(self, parent=None):
        super().__init__()

        self.layout_input = None
        self.language_input = None
        self.nav_bar = None
        self.pages = None
        self.general_page = None
        self.info_page = None
        self.languages = {
            "en": _("English"),
            "de": _("German"),
            "fr": _("French"),
        }
        self.app_layouts = {
            "system": _("System"),
            "light": _("Light"),
            "dark": _("Dark"),
        }
        self.btn_info = None
        self.btn_general = None

        self.icon_folder = "light" if is_dark_mode() else "dark"
        self.parent = parent
        self.init_ui()

    def init_ui(self) -> None:
        """
        Initializes the UI components and menu navigation with a real vertical sidebar.
        """
        self.setWindowTitle(_("System Settings"))
        self.setGeometry(100, 100, 800, 500)

        nav_buttons = {
            "general": ("settings-2-line.svg", _("General")),
            "info": ("information-line.svg", _("Info")),
        }

        main_layout = QHBoxLayout()

        splitter = QSplitter(Qt.Orientation.Horizontal)
        self.left_widget = QWidget()
        self.left_layout = QVBoxLayout(self.left_widget)
        self.left_layout.setContentsMargins(0, 0, 0, 0)
        self.left_widget.setMinimumWidth(50)
        self.left_widget.setMaximumWidth(200)

        self.nav_bar = Sidebar(nav_buttons, self.left_widget)
        self.nav_bar.action_triggered.connect(self.set_active_page)
        self.left_layout.addWidget(self.nav_bar)
        self.left_layout.addStretch()
        splitter.addWidget(self.left_widget)

        self.pages = QStackedWidget()

        self.general_page = self.create_general_page()
        self.info_page = self.create_info_page()

        self.pages.addWidget(self.general_page)
        self.pages.addWidget(self.info_page)

        splitter.addWidget(self.pages)
        splitter.setSizes([50, 750])

        splitter.setCollapsible(0, False)  # Left Panel is NOT collapsible
        splitter.setCollapsible(1, False)  # Center Panel is NOT collapsible

        splitter.setStretchFactor(
            0, 0
        )  # Left panel remains at min width unless expanded
        splitter.setStretchFactor(1, 1)  # Center panel takes priority in resizing

        splitter.setStyleSheet(
            """
            QSplitter::handle {
                background-color: #000;  /* Light Gray */
                width: 1px;  /* Make divider thicker */
            }
        """
        )

        main_layout.addWidget(splitter)
        self.nav_bar.set_active_action("general")
        self.setLayout(main_layout)

    def set_active_page(self, page) -> None:
        if page == "general":
            self.pages.setCurrentWidget(self.general_page)
        elif page == "info":
            self.pages.setCurrentWidget(self.info_page)

    def create_general_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        self.set_page_title(layout, _("General Settings"))

        form_box = QVBoxLayout()
        form_layout = self.get_form()

        self.language_input = self.get_form_field(QComboBox(), "QComboBox")
        for lang, lang_name in self.languages.items():
            self.language_input.addItem(lang_name, lang)

        self.layout_input = self.get_form_field(QComboBox(), "QComboBox")
        for app_layout, app_layout_name in self.app_layouts.items():
            self.layout_input.addItem(app_layout_name, app_layout)

        form_layout.addRow(_("Language:"), self.language_input)
        form_layout.addRow(_("Layout:"), self.layout_input)

        form_box.addLayout(form_layout)
        form_box.addStretch(1)

        action_bar = DialogActionBar(
            cancel_action=self.close,
            submit_action=self.save_general_settings,
            submit_label="Save",
        )

        form_box.addWidget(action_bar)
        layout.addLayout(form_box)
        page.setLayout(layout)
        self._set_form_data()
        return page

    def create_info_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout()

        self.set_page_title(layout, _("System Info"))
        value_color = THEME.ACCENT_COLOR if is_dark_mode() else THEME.MAIN_COLOR
        infos = self.get_infos()
        # loop through the activity data and display the data
        for index, (key, value) in enumerate(infos.items()):
            key_label = QLabel(_(key))
            font = key_label.font()
            font.setPointSize(14)
            key_label.setFont(font)
            key_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

            value_label = QLabel(str(value))
            font = value_label.font()
            font.setPointSize(14)
            value_label.setFont(font)
            value_label.setStyleSheet(f"color: {value_color};")
            value_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

            row_layout = QHBoxLayout()
            row_layout.setContentsMargins(0, 0, 0, 0)
            row_layout.addWidget(key_label)
            row_layout.addStretch()
            row_layout.addWidget(value_label)

            row_widget = QWidget()
            row_widget.setLayout(row_layout)
            layout.addWidget(row_widget)

            # Add a fine separator line (except after the last row)
            if index < len(infos.items()) - 1:
                separator = QFrame()
                separator.setFrameShape(QFrame.Shape.HLine)
                separator.setFrameShadow(QFrame.Shadow.Sunken)
                separator.setStyleSheet(f"color: {THEME.SYSTEM_BUTTON}")
                layout.addWidget(separator)
        layout.addStretch(1)
        page.setLayout(layout)
        return page

    @staticmethod
    def get_form_field(field, styleObject):
        field.setMinimumWidth(200)
        field.setStyleSheet(
            f"""
            {styleObject} {{
            padding: 5px;
            height: 28px;
            }}"""
        )
        return field

    @staticmethod
    def get_form() -> QFormLayout:
        form_layout = QFormLayout()
        form_layout.setAlignment(Qt.AlignmentFlag.AlignLeft)
        form_layout.setSpacing(20)
        form_layout.setFieldGrowthPolicy(
            QFormLayout.FieldGrowthPolicy.ExpandingFieldsGrow
        )
        form_layout.setFormAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
        )
        form_layout.setContentsMargins(0, 10, 0, 10)
        return form_layout

    def save_general_settings(self) -> None:
        settings = {
            "language": self.language_input.currentData(),
            "layout": self.layout_input.currentData(),
        }
        save_settings(settings)

    @staticmethod
    def set_page_title(layout, title: str) -> None:
        title_label = QLabel(title)
        title_label.setStyleSheet(
            """
                font-size: 16px;
                font-weight: bold;
            """
        )
        layout.addWidget(title_label)

    @staticmethod
    def get_infos():
        folder = os.path.expanduser("~/RunningData/media")
        media_count, media_size_mb = media_stats(folder)
        total_size_mb = folder_size(os.path.expanduser("~/RunningData"))
        info = {
            "app_path": os.path.expanduser("~/RunningData"),
            "media_count": f"{media_count} {_('files')}",
            "media_size": f"{round(media_size_mb, 2)} MB",
            "total_size": f"{round(total_size_mb, 2)} MB",
        }
        return info

    def _set_form_data(self):
        settings = load_settings_config()
        for i in range(self.language_input.count()):
            if self.language_input.itemData(i) == settings["language"]:
                self.language_input.setCurrentIndex(i)
                break
        for i in range(self.layout_input.count()):
            if self.layout_input.itemData(i) == settings["layout"]:
                self.layout_input.setCurrentIndex(i)
                break
