import os
import webbrowser

import cv2
from PyQt6.QtCore import QRect, Qt
from PyQt6.QtGui import QImage, QPixmap, QTextOption
from PyQt6.QtSvgWidgets import QSvgWidget
from PyQt6.QtWidgets import (
    QDialog,
    QDialogButtonBox,
    QFileDialog,
    QFrame,
    QHBoxLayout,
    QLabel,
    QMessageBox,
    QPushButton,
    QStackedWidget,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

from database.database_handler import DatabaseHandler
from utils.translations import _


def create_separator():
    """Creates a horizontal line separator."""
    separator = QFrame()
    separator.setFrameShape(QFrame.Shape.HLine)  # Horizontal line
    separator.setFrameShadow(QFrame.Shadow.Sunken)  # Slight depth effect
    separator.setStyleSheet("color: white;")  # White line
    return separator


class RunDetailsWindow(QDialog):
    def __init__(self, run_data, media_dir, db_handler: DatabaseHandler):
        super().__init__()
        self.current_page = 0
        self.db = db_handler
        self.media_dir = media_dir
        self.run_id = run_data["id"]
        self.activity_id = run_data["activity_id"]
        self.activity_type = run_data["activity_type"]
        self.comment = run_data["comment"]

        self.track_img_path = (
            run_data["track_img"] if run_data["track_img"] else None
        )  # Track image path
        self.elevation_img_path = (
            run_data["elevation_img"] if run_data["elevation_img"] else None
        )  # Elevation image path
        self.map_html = (
            run_data["map_html"] if run_data["map_html"] else None
        )  # Map HTML file

        self.current_index = 0
        self.media_files = self.db.get_media_files(self.activity_id)
        self.init_ui(run_data)

    def init_ui(self, run_data):
        """Setup UI elements for displaying run details with a structured layout."""
        self.setWindowTitle(
            _("{type} on {date} ").format(
                type=_(run_data["activity_type"]), date=run_data["date"]
            )
        )
        self.setGeometry(100, 100, 1200, 800)

        # ==== MAIN LAYOUT ====
        main_layout = QHBoxLayout()

        # ==== LEFT MENU ====
        menu_layout = QVBoxLayout()
        menu_layout.setAlignment(Qt.AlignmentFlag.AlignTop)
        menu_layout.setSpacing(5)

        show_map_btn = self.create_menu_button(_("Show Map"), self.open_map_in_browser)
        edit_comment_btn = self.create_menu_button(_("Edit Comment"), self.edit_comment)
        upload_media_btn = self.create_menu_button(_("Add Media"), self.upload_media)

        # Add to layout with dividers
        menu_layout.addWidget(show_map_btn)
        menu_layout.addWidget(create_separator(), 0)
        menu_layout.addWidget(edit_comment_btn)
        menu_layout.addWidget(create_separator(), 0)
        menu_layout.addWidget(upload_media_btn)

        # Left side frame for menu
        menu_frame = QFrame()
        menu_frame.setLayout(menu_layout)
        menu_frame.setFixedWidth(160)
        menu_frame.setStyleSheet(
            """
            border-right: 1px solid #555;
            padding-right: 10px;
        """
        )

        main_layout.addWidget(menu_frame)

        # ==== RIGHT CONTENT ====
        content_layout = QVBoxLayout()

        # ==== TITLE (Left-Aligned) ====
        title_label = QLabel(run_data["title"])
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        title_label.setStyleSheet(
            """
            font-size: 20px;
            font-weight: bold;
            padding: 5px 0px;
            margin: 0px 0px 0px -5px;
        """
        )

        # Create a horizontal line below the title
        title_separator = QFrame()
        title_separator.setFrameShape(QFrame.Shape.HLine)  # Horizontal line
        title_separator.setFrameShadow(QFrame.Shadow.Sunken)  # Slight depth effect
        title_separator.setStyleSheet("color: white;")  # White line

        # Layout
        title_layout = QVBoxLayout()
        title_layout.addWidget(title_label)
        title_layout.addWidget(title_separator)  # Add the separator line

        content_layout.addLayout(title_layout)

        # ==== COMMENT (Left-Aligned) ====
        comment_layout = QHBoxLayout()
        self.comment_label = QLabel(run_data["comment"] if run_data["comment"] else "")
        self.comment_label.setAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop
        )
        self.comment_label.setWordWrap(True)
        self.comment_label.setStyleSheet("border: none; padding: 0")
        comment_layout.addWidget(self.comment_label)
        content_layout.addLayout(comment_layout)

        # ==== CAROUSEL CONTAINER ====
        self.carousel_container = QWidget()  # Main container for carousel items
        self.carousel_layout = QHBoxLayout(self.carousel_container)
        self.carousel_layout.setAlignment(Qt.AlignmentFlag.AlignLeft)
        self.carousel_layout.setContentsMargins(0, 10, 0, 0)
        self.carousel_layout.setSpacing(0)

        carousel_wrapper = QVBoxLayout()  # Wrapper for carousel + navigation buttons
        carousel_wrapper.addWidget(self.carousel_container)

        # Navigation buttons
        nav_layout = QHBoxLayout()
        self.prev_button = QPushButton(_("◀ Previous"))
        self.prev_button.clicked.connect(self.prev_media)
        self.next_button = QPushButton(_("Next ▶"))
        self.next_button.clicked.connect(self.next_media)
        nav_layout.addWidget(self.prev_button)
        nav_layout.addWidget(self.next_button)

        carousel_wrapper.addLayout(nav_layout)
        content_layout.addLayout(carousel_wrapper)

        # ==== DATA, MAP, AND ELEVATION ====
        fourth_row_layout = QHBoxLayout()
        left_data_layout = QVBoxLayout()

        fields = [
            "time",
            "distance",
            "duration",
            "elevation_gain",
            "avg_speed",
            "avg_steps",
            "total_steps",
            "avg_power",
            "avg_heart_rate",
            "avg_pace",
            "fastest_pace",
            "slowest_pace",
            "pause",
        ]

        for field in fields:
            if field in run_data and run_data[field]:
                label = QLabel(f"<b>{_(field)}:</b> {run_data[field]}")
                left_data_layout.addWidget(label)

        left_data_layout.addStretch()
        fourth_row_layout.addLayout(left_data_layout, 1)

        middle_layout = QVBoxLayout()
        if os.path.exists(self.track_img_path):
            track_pixmap = QPixmap(self.track_img_path).scaled(250, 250)
            track_display = QLabel()
            track_display.setPixmap(track_pixmap)
            track_display.setAlignment(Qt.AlignmentFlag.AlignCenter)
            middle_layout.addWidget(track_display)
            middle_layout.addStretch()

        fourth_row_layout.addLayout(middle_layout, 1)

        right_layout = QVBoxLayout()
        if os.path.exists(self.elevation_img_path):
            elevation_display = QSvgWidget(self.elevation_img_path)
            elevation_display.setFixedSize(500, 250)
            right_layout.addWidget(elevation_display)
            right_layout.addStretch()

        fourth_row_layout.addLayout(right_layout, 1)
        content_layout.addLayout(fourth_row_layout, 1)
        content_layout.addStretch()

        main_layout.addLayout(content_layout)
        self.setLayout(main_layout)

        self.load_carousel_media()

    def edit_comment(self):
        """Opens a dialog to edit the comment with multiline support, dynamically adjusting width."""
        dialog = QDialog(self)
        dialog.setWindowTitle(_("Edit Comment"))

        # ✅ Set dialog width dynamically based on window size
        dialog_width = max(
            300, int(self.width() * 0.5)
        )  # 50% of window width, min 300px
        dialog.setFixedSize(dialog_width, 300)  # Fixed height, dynamic width

        layout = QVBoxLayout()
        text_edit = QTextEdit()
        text_edit.setText(self.comment)
        text_edit.setWordWrapMode(
            QTextOption.WrapMode.WordWrap
        )  # ✅ Enable auto line breaks
        text_edit.setFixedSize(dialog_width - 40, 200)  # Keep within dialog bounds

        # Buttons
        button_box = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        button_box.accepted.connect(dialog.accept)
        button_box.rejected.connect(dialog.reject)

        layout.addWidget(text_edit)
        layout.addWidget(button_box)
        dialog.setLayout(layout)

        if dialog.exec():
            new_comment = text_edit.toPlainText().strip()
            if new_comment:
                self.comment = new_comment
                self.comment_label.setText(self.comment)
                self.db.update_comment(self.activity_id, self.comment)

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

    def open_map_in_browser(self):
        """Opens the map HTML file in the default web browser."""
        if self.map_html and os.path.exists(self.map_html):
            webbrowser.open(f"file://{os.path.abspath(self.map_html)}")
        else:
            QMessageBox.warning(self, "Map Error", "Map file not found.")

    def process_image_for_thumbnail(self, pixmap, width, height):
        original_size = pixmap.size()

        if original_size.width() > width and original_size.height() > height:
            crop_size = min(original_size.width(), original_size.height())
            x_offset = (original_size.width() - crop_size) // 2
            y_offset = (original_size.height() - crop_size) // 2
            cropped_pixmap = pixmap.copy(
                QRect(x_offset, y_offset, crop_size, crop_size)
            )
            return cropped_pixmap.scaled(
                width,
                height,
                Qt.AspectRatioMode.IgnoreAspectRatio,
                Qt.TransformationMode.SmoothTransformation,
            )

        else:
            # ✅ Scale up if smaller
            return pixmap.scaled(
                width,
                height,
                Qt.AspectRatioMode.IgnoreAspectRatio,
                Qt.TransformationMode.SmoothTransformation,
            )

    def create_menu_button(self, text, callback):
        """Creates an invisible menu button with padding."""
        btn = QPushButton(text)
        btn.clicked.connect(callback)
        btn.setStyleSheet(
            """
            QPushButton {
                background: transparent;
                border: none;
                font-size: 16px;
                padding: 10px 0;
                color: white;
                text-align: left;
            }
            QPushButton:hover {
                color: #cccccc;
            }
        """
        )
        btn.setCursor(Qt.CursorShape.PointingHandCursor)  # ✅ Show hand cursor on hover
        return btn

    def open_media(self, item):
        """Opens selected media file (image or video)."""
        file_path = item.data(Qt.ItemDataRole.UserRole)
        if os.path.exists(file_path):
            webbrowser.open(f"file://{os.path.abspath(file_path)}")
        else:
            QMessageBox.warning(self, "Media Error", "Media file not found.")

    def show_full_image(self, file_path):
        """Opens the full-size image in a popup window instead of a browser."""
        if not os.path.exists(file_path):
            QMessageBox.warning(self, "Image Error", "Image file not found.")
            return

        dialog = QDialog(self)
        dialog.setWindowTitle("Full Image ViewMode")
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
            QMessageBox.warning(self, "Video Error", "Video file not found.")

    def load_carousel_media(self):
        items_per_page = 5
        # Clear existing carousel items
        while self.carousel_layout.count():
            widget = self.carousel_layout.takeAt(0).widget()
            if widget:
                widget.deleteLater()

        if not self.media_files:
            self.prev_button.setVisible(False)
            self.next_button.setVisible(False)
            return

        self.carousel_layout.setSpacing(10)  #
        total_pages = (len(self.media_files) + items_per_page - 1) // items_per_page
        start_index = self.current_page * items_per_page
        end_index = start_index + items_per_page
        displayed_media = self.media_files[start_index:end_index]

        for media in displayed_media:
            media_type, file_path = media[1], media[2]

            # Create container for media (image/video + delete button)
            media_container = QFrame()
            media_container.setFixedSize(200, 200)
            media_container.setStyleSheet(
                "border: none; background-color: transparent; position: relative;"
            )

            # Stacked layout to hold media content
            stacked_layout = QStackedWidget(media_container)
            stacked_layout.setFixedSize(200, 200)

            # Media Display (Image/Video)
            media_label = QLabel()
            media_label.setFixedSize(200, 200)

            if media_type == "image":
                pixmap = QPixmap(file_path)
                processed_pixmap = self.process_image_for_thumbnail(pixmap, 200, 200)
                media_label.setPixmap(processed_pixmap)
                media_label.mousePressEvent = (
                    lambda event, path=file_path: self.show_full_image(path)
                )

            elif media_type == "video":
                thumbnail = self.get_video_thumbnail(file_path)
                media_label.setPixmap(thumbnail)
                media_label.mousePressEvent = (
                    lambda event, path=file_path: self.play_video(path)
                )

            media_label.setStyleSheet("border-radius: 5px;")  # Smooth edges
            stacked_layout.addWidget(media_label)  # Add media

            # DELETE BUTTON (Top-Right Overlay)
            delete_btn = QPushButton("x")
            delete_btn.setFixedSize(24, 24)
            delete_btn.setStyleSheet(
                "background-color: black; color: white; border-radius: 12px; font-weight: bold;"
                "font-size: 12px; border: none; position: absolute;"
            )
            delete_btn.clicked.connect(
                lambda checked, path=file_path: self.delete_media(path)
            )

            # Set absolute positioning of delete button inside media_container
            delete_btn.setParent(media_container)
            delete_btn.move(170, 5)  # ✅ Moves to the top-right corner

            # Ensure delete button is clickable
            delete_btn.raise_()  # ✅ Ensures it's on top
            delete_btn.setAttribute(
                Qt.WidgetAttribute.WA_TransparentForMouseEvents, False
            )
            media_label.setAttribute(
                Qt.WidgetAttribute.WA_TransparentForMouseEvents, False
            )

            # Final layout inside media container
            final_layout = QVBoxLayout(media_container)
            final_layout.setContentsMargins(0, 0, 0, 0)  # ✅ Remove extra margins
            final_layout.addWidget(stacked_layout)

            # Add media container to the carousel
            self.carousel_layout.addWidget(media_container)

        # Enable/Disable navigation buttons
        self.prev_button.setEnabled(self.current_page > 0)
        self.next_button.setEnabled(self.current_page < total_pages - 1)
        self.prev_button.setVisible(total_pages > 1)
        self.next_button.setVisible(total_pages > 1)

    def delete_media(self, file_path):
        """Deletes the media entry from the database and removes the file."""
        confirm = QMessageBox.question(
            self,
            "Delete Media",
            "Are you sure you want to delete this media?",
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

    def get_video_thumbnail(self, video_path, width=300, height=300):
        """Extracts the first frame of a video and returns a QPixmap thumbnail."""
        cap = cv2.VideoCapture(video_path)
        success, frame = cap.read()
        cap.release()

        if success:
            frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            h, w, ch = frame.shape
            bytes_per_line = ch * w
            qimage = QImage(
                frame.data, w, h, bytes_per_line, QImage.Format.Format_RGB888
            )
            pixmap = QPixmap.fromImage(qimage)

            return self.process_image_for_thumbnail(pixmap, width, height)

        else:
            # If thumbnail generation fails, return a default icon
            default_pixmap = QPixmap(width, height)
            default_pixmap.fill(Qt.GlobalColor.gray)
            return default_pixmap
