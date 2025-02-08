from PyQt6.QtSvgWidgets import QSvgWidget
from PyQt6.QtWidgets import (
    QDialog, QLabel, QVBoxLayout, QHBoxLayout, QTextEdit, QPushButton, QFileDialog, QWidget, QMessageBox
)
from PyQt6.QtGui import QPixmap
from PyQt6.QtCore import Qt
import os
import webbrowser

from processing.database_handler import update_comment, update_photo


class RunDetailsWindow(QDialog):
    def __init__(self, run_data):
        super().__init__()
        self.run_id = run_data[0]  # Run ID for database updates
        self.activity_type = run_data[17]  # Activity Type
        self.date = run_data[1]  # Date
        self.track_img_path = run_data[18] if len(run_data) > 18 else None  # Track image path
        self.elevation_img_path = run_data[19] if len(run_data) > 19 else None  # Elevation image path
        self.map_html = run_data[20] if len(run_data) > 20 else None  # Map HTML file
        self.comment = run_data[21] if len(run_data) > 21 else ""  # Comment from database
        self.photo_path = run_data[22] if len(run_data) > 22 else None  # Uploaded photo

        self.initUI(run_data)

    def initUI(self, run_data):
        """Setup UI elements for displaying run details with a structured layout."""
        self.setWindowTitle(f"Run Details - {self.activity_type} on {self.date}")
        self.setGeometry(100, 100, 1000, 800)  # Adjusted size for structured layout

        # ==== MAIN LAYOUT ====
        main_layout = QVBoxLayout()

        # ==== TITLE ====
        title_label = QLabel(f"{self.activity_type} - {self.date}")
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        title_label.setStyleSheet("font-size: 18px; font-weight: bold; padding: 10px 0px;")
        main_layout.addWidget(title_label)

        # ==== ROW 1: Data | Plot + Show Map Button | Elevation Profile ====
        top_layout = QHBoxLayout()

        # LEFT: Run Data (excluding Activity Type & Date)
        left_top_layout = QVBoxLayout()
        headers = [
            "Start Time", "Distance (km)", "Total Time", "Elevation Gain (m)",
            "Avg Speed (km/h)", "Avg Steps (SPM)", "Total Steps", "Avg Power (Watts)",
            "Avg Heart Rate (BPM)", "Avg Pace", "Fastest Pace", "Slowest Pace", "Pause"
        ]

        db_columns = [
            4,  # Start Time
            5,  # Distance (km)
            6,  # Total Time
            7,  # Elevation Gain (m)
            8,  # Avg Speed (km/h)
            9,  # Avg Steps (SPM)
            10,  # Total Steps
            11,  # Avg Power (Watts)
            12,  # Avg Heart Rate (BPM)
            13,  # Avg Pace
            14,  # Fastest Pace
            15,  # Slowest Pace
            16  # Pause
        ]

        for header, db_index in zip(headers, db_columns):
            label = QLabel(f"<b>{header}:</b> {run_data[db_index]}")
            left_top_layout.addWidget(label)

        top_layout.addLayout(left_top_layout, 1)

        # MIDDLE: Track Plot + Show Map Button
        middle_top_layout = QVBoxLayout()
        if os.path.exists(self.track_img_path):
            track_pixmap = QPixmap(self.track_img_path).scaled(300, 300)
            track_display = QLabel()
            track_display.setPixmap(track_pixmap)
            track_display.setAlignment(Qt.AlignmentFlag.AlignCenter)
            middle_top_layout.addWidget(track_display)

        # Show Map Button
        show_map_btn = QPushButton("Show Map in Browser")
        show_map_btn.clicked.connect(self.open_map_in_browser)
        middle_top_layout.addWidget(show_map_btn, alignment=Qt.AlignmentFlag.AlignCenter)

        top_layout.addLayout(middle_top_layout, 1)

        # RIGHT: Elevation Profile
        if os.path.exists(self.elevation_img_path):
            elevation_display = QSvgWidget(self.elevation_img_path)
            elevation_display.setFixedSize(600, 300)
            top_layout.addWidget(elevation_display, 1)

        main_layout.addLayout(top_layout)

        # ==== ROW 2: Comment Box | Uploaded Image ====
        bottom_layout = QHBoxLayout()

        # LEFT: Comment Box
        left_bottom_layout = QVBoxLayout()

        self.comment_box = QTextEdit()
        self.comment_box.setPlaceholderText("Add your comment here...")
        self.comment_box.setText(self.comment if self.comment else "")
        left_bottom_layout.addWidget(self.comment_box)

        # Button to save comment
        save_comment_btn = QPushButton("Save Comment")
        save_comment_btn.clicked.connect(self.save_comment)
        left_bottom_layout.addWidget(save_comment_btn)

        bottom_layout.addLayout(left_bottom_layout, 1)

        # RIGHT: Uploaded Image (Photo)
        right_bottom_layout = QVBoxLayout()
        self.photo_display = QLabel()
        if self.photo_path and os.path.exists(self.photo_path):
            self.photo_display.setPixmap(QPixmap(self.photo_path).scaled(300, 300))
            self.photo_display.setAlignment(Qt.AlignmentFlag.AlignCenter)

        right_bottom_layout.addWidget(self.photo_display)

        # Button to upload a photo
        upload_photo_btn = QPushButton("Upload Photo")
        upload_photo_btn.clicked.connect(self.upload_photo)
        right_bottom_layout.addWidget(upload_photo_btn)

        bottom_layout.addLayout(right_bottom_layout, 1)

        main_layout.addLayout(bottom_layout)

        self.setLayout(main_layout)

    def save_comment(self):
        """Saves the comment to the database."""
        new_comment = self.comment_box.toPlainText()
        update_comment(self.run_id, new_comment)

    def upload_photo(self):
        """Opens a file dialog to upload and save a new photo."""
        file_path, _ = QFileDialog.getOpenFileName(self, "Select Photo", "", "Images (*.png *.jpg *.jpeg)")
        if file_path:
            save_photo_path = os.path.join("photos", f"run_{self.run_id}.jpg")  # Save to a dedicated folder
            os.rename(file_path, save_photo_path)  # Move photo

            # Update the database
            update_photo(self.run_id, save_photo_path)

            # Update UI
            self.photo_display.setPixmap(QPixmap(save_photo_path).scaled(300, 300))

    def open_map_in_browser(self):
        """Opens the map HTML file in the default web browser."""
        if self.map_html and os.path.exists(self.map_html):
            webbrowser.open(f"file://{os.path.abspath(self.map_html)}")
        else:
            QMessageBox.warning(self, "Map Error", "Map file not found.")