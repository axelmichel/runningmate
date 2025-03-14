# ruff: noqa
import locale
import os
import time
import webbrowser
from datetime import datetime

from PyQt6.QtCore import QDate, Qt, QTime
from PyQt6.QtGui import QPixmap
from PyQt6.QtWidgets import (
    QDateEdit,
    QDialog,
    QDoubleSpinBox,
    QFileDialog,
    QFormLayout,
    QFrame,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMessageBox,
    QPushButton,
    QSpinBox,
    QSplitter,
    QStackedWidget,
    QTextEdit,
    QTimeEdit,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from database.user_settings import UserSettings
from processing.system_settings import ViewMode
from ui.dialog_action_bar import DialogActionBar
from ui.dialog_detail_pages.page_zones import page_zones
from ui.side_bar import Sidebar
from ui.themes import THEME
from utils.image_thumbnail import image_thumbnail
from utils.translations import _
from utils.video_thumbnail import video_thumbnail


class DialogDetail(QDialog):
    def __init__(
        self,
        activity_id,
        activity_type,
        media_dir,
        db_handler: DatabaseHandler,
        user_settings: UserSettings,
        parent=None,
    ):
        super().__init__()
        self.items_per_page = None
        self.form_container = None
        self.form_layout = None
        self.title_input = None
        self.comment_input = None
        self.upload_button = None
        self.distance_input = None
        self.date_input = None
        self.time_input = None
        self.duration_input = None
        self.calories_input = None
        self.elevation_input = None
        self.segment_page = None
        self.carousel_layout = None
        self.prev_button = None
        self.next_button = None
        self.current_page = 0
        self.pages = None
        self.map_page = None
        self.general_page = None
        self.effect_page = None
        self.zones_page = None
        self.nav_bar = None
        self.left_layout = None
        self.left_widget = None
        self.activity_id = activity_id
        self.activity_type = activity_type
        self.parent = parent
        self.db = db_handler
        self.media_dir = media_dir
        self.activity = self.load_activity(activity_id, activity_type)
        self.media_files = self.db.get_media_files(activity_id)
        self.user = user_settings.get_user_data()
        self.init_ui()

    def init_ui(self):
        nav_buttons = {
            "general": ("information-line.svg", "General"),
            "segments": ("stack-fill.svg", "Segments"),
            "map": ("map-pin-fill.svg", "Map"),
            "effect": ("timer-flash-fill.svg", "Effect"),
            "zones": ("heart-pulse-line.svg", "Heart Rate Zones"),
            "edit": ("pencil-line.svg", "Edit"),
        }

        self.setWindowTitle(_("Activity Details"))
        self.setGeometry(100, 100, 800, 700)

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
        self.map_page = self.create_map_page()
        self.segment_page = self.create_segments_page()
        self.effect_page = self.create_effect_page()
        self.zones_page = self.create_zones_page()
        self.edit_page = self.create_edit_page()

        self.pages.addWidget(self.general_page)
        self.pages.addWidget(self.map_page)
        self.pages.addWidget(self.effect_page)
        self.pages.addWidget(self.zones_page)
        self.pages.addWidget(self.edit_page)

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

    def load_activity(self, activity_id, activity_type):
        data = None
        if activity_type == ViewMode.RUN:
            data = self.db.fetch_run_by_activity_id(activity_id)
        elif activity_type == ViewMode.WALK:
            data = self.db.fetch_walk_by_activity_id(activity_id)
        elif activity_type == ViewMode.CYCLE:
            data = self.db.fetch_ride_by_activity_id(activity_id)
        return data

    def set_active_page(self, page) -> None:
        if page == "general":
            self.pages.setCurrentWidget(self.general_page)
        elif page == "map":
            self.pages.setCurrentWidget(self.map_page)
        elif page == "effect":
            self.pages.setCurrentWidget(self.effect_page)
        elif page == "zones":
            self.pages.setCurrentWidget(self.zones_page)
        elif page == "edit":
            self.pages.setCurrentWidget(self.edit_page)

    def create_general_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        layout.addLayout(self.get_page_title(_("Overview")))

        comment_layout = QHBoxLayout()
        comment_label = QLabel(
            self.activity["comment"] if self.activity["comment"] else ""
        )
        comment_label.setAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop
        )
        comment_label.setWordWrap(True)
        comment_label.setStyleSheet("border: none; padding: 0")
        comment_layout.addWidget(comment_label)
        layout.addLayout(comment_layout)

        carousel = self.get_carousel()
        layout.addLayout(carousel)

        layout.addStretch(1)
        page.setLayout(layout)
        self.load_carousel_media()
        return page

    def get_carousel(self):
        carousel_container = QWidget()  # Main container for carousel items
        self.carousel_layout = QHBoxLayout(carousel_container)
        self.carousel_layout.setAlignment(Qt.AlignmentFlag.AlignLeft)
        self.carousel_layout.setContentsMargins(0, 10, 0, 0)
        self.carousel_layout.setSpacing(0)

        carousel_wrapper = QVBoxLayout()  # Wrapper for carousel + navigation buttons
        carousel_wrapper.addWidget(carousel_container)

        nav_layout = QHBoxLayout()
        self.prev_button = QPushButton(_("◀ Previous"))
        self.prev_button.clicked.connect(self.prev_media)
        self.next_button = QPushButton(_("Next ▶"))
        self.next_button.clicked.connect(self.next_media)
        nav_layout.addWidget(self.prev_button)
        nav_layout.addWidget(self.next_button)

        carousel_wrapper.addLayout(nav_layout)
        return carousel_wrapper

    def create_map_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        layout.addLayout(self.get_page_title(_("Map")))
        layout.addStretch(1)
        page.setLayout(layout)
        return page

    def create_effect_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        layout.addLayout(self.get_page_title(_("Trainings Effect")))
        layout.addStretch(1)
        page.setLayout(layout)
        return page

    def create_segments_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        layout.addLayout(self.get_page_title(_("Segments")))
        layout.addStretch(1)
        page.setLayout(layout)
        return page

    def create_zones_page(self):
        return page_zones(
            self.get_page_title(_("Heart Rate Zones")),
            self.db,
            self.user["id"],
            self.activity_id,
            self.activity_type,
            self.activity["raw_date"],
        )

    def get_form_field(self, field, styleObject):
        field.setMinimumWidth(200)
        field.setStyleSheet(
            f"""
            {styleObject} {{
            padding: 5px;
            height: 28px;
            }}"""
        )
        return field

    def get_form(self) -> QWidget:
        self.form_container = QWidget()
        self.form_container.setObjectName(
            "formContainer"
        )  # Set object name for styling

        self.form_layout = QFormLayout(self.form_container)
        self.form_layout.setAlignment(Qt.AlignmentFlag.AlignLeft)
        self.form_layout.setSpacing(20)
        self.form_layout.setFieldGrowthPolicy(
            QFormLayout.FieldGrowthPolicy.ExpandingFieldsGrow
        )
        self.form_layout.setFormAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
        )
        self.form_layout.setContentsMargins(0, 10, 0, 10)
        return self.form_container

    def create_edit_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(10, 10, 10, 10)
        layout.addLayout(self.get_page_title(_("Edit")))

        layout.addWidget(QLabel("Title:"))
        self.title_input = QLineEdit(self.activity["title"])
        layout.addWidget(self.title_input)

        layout.addWidget(QLabel("Comment:"))
        self.comment_input = QTextEdit(self.activity["comment"])
        layout.addWidget(self.comment_input)

        self.upload_button = QPushButton("Add Media")
        self.upload_button.clicked.connect(self.upload_media)
        layout.addWidget(self.upload_button)

        form_box = QVBoxLayout()
        form_widget = self.get_form()  # Get the QWidget containing the form
        form_layout = form_widget.layout()  #
        self.distance_input = self.get_form_field(QDoubleSpinBox(), "QDoubleSpinBox")
        self.distance_input.setRange(0, 1000)
        self.distance_input.setDecimals(2)

        self.date_input = self.get_form_field(QDateEdit(), "QDateEdit")
        self.date_input.setDisplayFormat("dd.MM.yyyy")

        self.time_input = self.get_form_field(QTimeEdit(), "QTimeEdit")
        self.time_input.setDisplayFormat("HH:mm")
        self.duration_input = self.get_form_field(QTimeEdit(), "QTimeEdit")
        self.duration_input.setDisplayFormat("HH:mm:ss")

        self.calories_input = self.get_form_field(QSpinBox(), "QSpinBox")
        self.calories_input.setRange(0, 10000)

        self.elevation_input = self.get_form_field(QDoubleSpinBox(), "QDoubleSpinBox")
        self.elevation_input.setRange(0, 10000)
        self.elevation_input.setDecimals(2)

        self.setStyleSheet(
            """
            #formContainer QLabel {
                padding: 5px 0px;
                min-height: 28px;
                line-height: 28px;
            }
        """
        )

        form_layout.addRow(_("Distance:"), self.distance_input)
        form_layout.addRow(_("Date:"), self.date_input)
        form_layout.addRow(_("Time:"), self.time_input)
        form_layout.addRow(_("Duration:"), self.duration_input)
        form_layout.addRow(_("Elevation:"), self.elevation_input)

        layout.addWidget(QLabel(_("Edit Activity Details")))

        date_str = str(self.activity["date"])
        qdate = QDate.fromString(date_str, "dd.MM.yyyy")
        if qdate.isValid():
            self.date_input.setDate(qdate)
        else:
            self.date_input.setDate(QDate.currentDate())

        time_str = str(self.activity["time"])
        hours, minutes = map(int, time_str.split(":"))
        self.time_input.setTime(QTime(hours, minutes))

        duration_str = self.activity["duration"]
        hours, minutes, seconds = map(int, duration_str.split(":"))

        self.duration_input.setTime(QTime(hours, minutes, seconds))
        self.calories_input.setValue(self.activity["calories"])
        self.elevation_input.setValue(self.activity["elevation_gain"])
        self.distance_input.setValue(self.activity["distance"])

        form_box.addWidget(form_widget)
        form_box.addStretch(1)

        action_bar = DialogActionBar(
            cancel_action=self.close,
            submit_action=self.update_activity,
            submit_label="Save",
        )

        form_box.addWidget(action_bar)
        layout.addLayout(form_box)
        page.setLayout(layout)
        return page

    def update_activity(self):

        data = {
            "id": self.activity_id,
            "title": self.title_input.text(),
            "comment": self.comment_input.toPlainText()
        }

        self.db.update_activity_data(data)
        self.activity = self.load_activity(self.activity_id, self.activity_type)

    def upload_media(self):
        """Allows the user to upload media files."""
        file_paths, _ = QFileDialog.getOpenFileNames(
            self,
            "Select Media Files",
            "",
            "Images/Videos (*.png *.jpg *.jpeg *.mp4 *.avi)",
        )
        if file_paths:
            for file_path in file_paths:
                file_name = os.path.basename(file_path)
                media_type = (
                    "image"
                    if file_name.lower().endswith((".png", ".jpg", ".jpeg"))
                    else "video"
                )
                save_path = os.path.join(self.media_dir, file_name)
                os.rename(file_path, save_path)
                self.db.insert_media(self.activity_id, media_type, save_path)

            self.media_files = self.db.get_media_files(self.activity_id)
            self.load_carousel_media()

    def get_page_title(self, title: str) -> QVBoxLayout:
        """Creates a formatted title bar with a date, activity title, and right-aligned page title."""
        locale.setlocale(
            locale.LC_TIME, "en_US.UTF-8"
        )  # Ensures month & weekday names are in English

        # Convert Unix timestamp to datetime
        activity_date = datetime.fromtimestamp(self.activity["raw_date"])
        formatted_date = activity_date.strftime(
            "%a, %d.%m.%y"
        )  # Example: "Monday, 01.01.25"

        # Create labels
        date_label = QLabel(f"{formatted_date} - {self.activity['title']}")
        date_label.setStyleSheet("font-size: 14px; font-weight: bold;")
        date_label.setAlignment(Qt.AlignmentFlag.AlignLeft)

        title_label = QLabel(title)
        title_label.setStyleSheet(
            f"font-size: 14px; font-weight: bold; color:{THEME.ACCENT_COLOR};"
        )
        title_label.setAlignment(Qt.AlignmentFlag.AlignRight)

        # Layout setup
        title_layout = QHBoxLayout()
        title_layout.addWidget(date_label)  # Left side: Date + Activity title
        title_layout.addStretch()  # Stretch: Pushes right-aligned text
        title_layout.addWidget(title_label)  # Right side: Page title

        # Wrap layout inside a vertical box (ensures proper margins)
        main_layout = QVBoxLayout()
        main_layout.addLayout(title_layout)
        main_layout.setContentsMargins(0, 0, 0, 10)  # Add bottom margin

        return main_layout

    def open_media(self, item):
        """Opens selected media file (image or video)."""
        file_path = item.data(Qt.ItemDataRole.UserRole)
        if os.path.exists(file_path):
            webbrowser.open(f"file://{os.path.abspath(file_path)}")
        else:
            QMessageBox.warning(self, _("Media Error"), _("Media file not found."))

    def show_full_image(self, file_path):
        """Opens the full-size image in a popup window instead of a browser."""
        if not os.path.exists(file_path):
            QMessageBox.warning(self, _("Image Error"), _("Image file not found."))
            return

        dialog = QDialog(self)
        dialog.setWindowTitle(_("Media Viewer"))
        dialog.setGeometry(100, 100, 800, 600)  # Set window size

        layout = QVBoxLayout()
        image_label = QLabel()

        pixmap = QPixmap(file_path)
        if not pixmap.isNull():
            scaled_pixmap = pixmap.scaled(
                800,
                600,
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation,
            )
            image_label.setPixmap(scaled_pixmap)

        layout.addWidget(image_label)
        dialog.setLayout(layout)
        dialog.exec()  # Show the dialog window

    def play_video(self, file_path):
        """Plays the video when clicked."""
        if os.path.exists(file_path):
            webbrowser.open(f"file://{os.path.abspath(file_path)}")
        else:
            QMessageBox.warning(self, _("Video Error"), _("Video file not found."))

    def load_carousel_media(self):
        self.items_per_page = 5
        while self.carousel_layout.count():
            widget = self.carousel_layout.takeAt(0).widget()
            if widget:
                widget.deleteLater()

        if not self.media_files:
            self.prev_button.setVisible(False)
            self.next_button.setVisible(False)
            return
        print(len(self.media_files))
        self.carousel_layout.setSpacing(10)  #
        total_pages = (len(self.media_files) + self.items_per_page - 1)
        start_index = self.current_page * self.items_per_page
        end_index = start_index + self.items_per_page
        displayed_media = self.media_files[start_index:end_index]

        for media in displayed_media:
            media_type, file_path = media[1], media[2]

            media_container = QFrame()
            media_container.setFixedSize(200, 200)
            media_container.setStyleSheet(
                "border: none; background-color: transparent; position: relative;"
            )

            stacked_layout = QStackedWidget(media_container)
            stacked_layout.setFixedSize(200, 200)

            media_label = QLabel()
            media_label.setFixedSize(200, 200)

            if media_type == "image":
                pixmap = QPixmap(file_path)
                processed_pixmap = image_thumbnail(pixmap, 200, 200)
                media_label.setPixmap(processed_pixmap)
                media_label.mousePressEvent = (
                    lambda event, path=file_path: self.show_full_image(path)
                )

            elif media_type == "video":
                thumbnail = video_thumbnail(file_path)
                media_label.setPixmap(thumbnail)
                media_label.mousePressEvent = (
                    lambda event, path=file_path: self.play_video(path)
                )

            media_label.setStyleSheet("border-radius: 5px;")  # Smooth edges
            stacked_layout.addWidget(media_label)  # Add media

            delete_btn = QPushButton("x")
            delete_btn.setFixedSize(24, 24)
            delete_btn.setStyleSheet(
                "background-color: black; color: white; border-radius: 12px; font-weight: bold;"
                "font-size: 12px; border: none; position: absolute;"
            )
            delete_btn.clicked.connect(
                lambda checked, path=file_path: self.delete_media(path)
            )

            delete_btn.setParent(media_container)
            delete_btn.move(170, 5)

            delete_btn.raise_()
            delete_btn.setAttribute(
                Qt.WidgetAttribute.WA_TransparentForMouseEvents, False
            )
            media_label.setAttribute(
                Qt.WidgetAttribute.WA_TransparentForMouseEvents, False
            )

            final_layout = QVBoxLayout(media_container)
            final_layout.setContentsMargins(0, 0, 0, 0)
            final_layout.addWidget(stacked_layout)

            self.carousel_layout.addWidget(media_container)

        self.prev_button.setEnabled(self.current_page > 0)
        self.next_button.setEnabled(self.current_page < total_pages - 1)
        self.prev_button.setVisible(total_pages > 1)
        self.next_button.setVisible(total_pages > 1)

    def delete_media(self, file_path):
        """Deletes the media entry from the database and removes the file."""
        confirm = QMessageBox.question(
            self,
            _("Delete Media"),
            _("Are you sure you want to delete this media?"),
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )

        if confirm == QMessageBox.StandardButton.Yes:
            # Remove from database
            self.db.delete_media(self.activity_id, file_path)

            # Remove file from storage
            if os.path.exists(file_path):
                os.remove(file_path)

            # Refresh carousel
            self.media_files = self.db.get_media_files(self.activity_id)
            self.current_page = 0
            self.load_carousel_media()

    def next_media(self):
        """Shows the next set of 5 media items."""
        items_per_page = 5
        total_pages = (len(self.media_files) + items_per_page - 1) // items_per_page
        if self.current_page < total_pages - 1:
            self.current_page += 1
            self.load_carousel_media()

    def prev_media(self):
        """Shows the previous set of 5 media items."""
        if self.current_page > 0:
            self.current_page -= 1
            self.load_carousel_media()
