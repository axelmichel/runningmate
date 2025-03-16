from PyQt6.QtCore import QDate, Qt, QTime
from PyQt6.QtGui import QPixmap
from PyQt6.QtWidgets import (
    QDateEdit,
    QDoubleSpinBox,
    QFormLayout,
    QFrame,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QSpinBox,
    QTextEdit,
    QTimeEdit,
    QVBoxLayout,
    QWidget,
)

from ui.dialog_action_bar import DialogActionBar
from utils.image_thumbnail import image_thumbnail
from utils.translations import _
from utils.video_thumbnail import video_thumbnail


class PageEdit:
    def __init__(self, activity, title, media, parent) -> None:
        self.form_layout = None
        self.media_layout = None
        self.form_container = None
        self.calories_input = None
        self.elevation_input = None
        self.duration_input = None
        self.time_input = None
        self.date_input = None
        self.distance_input = None
        self.upload_button = None
        self.comment_input = None
        self.title_input = None
        self.media_files = media
        self.parent = parent
        self.activity = activity
        self.title = title

    def get_page(self):
        page = QWidget()
        layout = QVBoxLayout()
        layout.setContentsMargins(20, 0, 0, 0)
        layout.addLayout(self.title)

        layout.addWidget(QLabel("Title:"))
        self.title_input = QLineEdit(self.activity["title"])
        self.title_input.setStyleSheet(
            """
            QLineEdit {
                padding: 5px;
                height: 28px;
            }
            """
        )
        layout.addWidget(self.title_input)

        layout.addWidget(QLabel("Comment:"))
        self.comment_input = QTextEdit(self.activity["comment"])
        layout.addWidget(self.comment_input)

        # ✅ Create Media Layout for Thumbnails
        self.media_layout = QHBoxLayout()
        self.media_layout.setSpacing(10)
        layout.addLayout(self.media_layout)

        self.list_media()  # Load existing media

        # ✅ Add Media Button
        self.upload_button = QPushButton("Add Media")
        self.upload_button.clicked.connect(self.parent.upload_media)
        layout.addWidget(self.upload_button)

        form_box = QVBoxLayout()
        form_widget = self.get_form()
        form_layout = form_widget.layout()

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

        form_layout.addRow(_("Distance:"), self.distance_input)
        form_layout.addRow(_("Date:"), self.date_input)
        form_layout.addRow(_("Time:"), self.time_input)
        form_layout.addRow(_("Duration:"), self.duration_input)
        form_layout.addRow(_("Elevation:"), self.elevation_input)

        sub_title = QLabel(_("Edit Activity Details"))
        layout.addStretch(1)
        layout.addWidget(sub_title)
        sub_title.setStyleSheet(
            """
            font-size: 13px;
            font-weight: bold;
            margin-bottom: 10px;
        """
        )

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
            cancel_action=self.parent.close,
            submit_action=self.parent.update_activity,
            submit_label="Save",
        )

        form_box.addWidget(action_bar)
        layout.addLayout(form_box)
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

    def get_form(self) -> QWidget:
        self.form_container = QWidget()
        self.form_container.setObjectName("formContainer")

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

    def list_media(self):
        """Adds media thumbnails to the layout, aligning them to the left."""
        if self.media_files:
            media_container = QWidget()
            media_layout = QHBoxLayout(media_container)
            media_layout.setAlignment(Qt.AlignmentFlag.AlignLeft)
            media_layout.setSpacing(5)

            for media in self.media_files:
                media_type, file_path = media[1], media[2]

                media_frame = QFrame()
                media_frame.setFixedSize(80, 80)
                media_frame.setStyleSheet(
                    "border: none; background-color: transparent;"
                )

                media_label = QLabel()
                media_label.setFixedSize(80, 80)

                if media_type == "image":
                    pixmap = QPixmap(file_path)
                    processed_pixmap = image_thumbnail(pixmap, 80, 80)
                    media_label.setPixmap(processed_pixmap)

                elif media_type == "video":
                    thumbnail = video_thumbnail(file_path)
                    media_label.setPixmap(thumbnail)

                media_label.setStyleSheet("border-radius: 5px;")
                media_frame_layout = QVBoxLayout()
                media_frame_layout.addWidget(media_label)
                media_frame_layout.setContentsMargins(0, 0, 0, 0)
                media_frame.setLayout(media_frame_layout)

                media_layout.addWidget(media_frame)

            media_layout.addStretch()

            self.media_layout.addWidget(media_container)

    def refresh_media(self, media):
        self.media_files = media

        while self.media_layout.count():
            item = self.media_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        self.list_media()
