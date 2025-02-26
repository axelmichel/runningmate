import datetime
import os
import shutil

from PyQt6.QtCore import pyqtSignal
from PyQt6.QtWidgets import (
    QDialog,
    QFileDialog,
    QHBoxLayout,
    QLabel,
    QMessageBox,
    QProgressBar,
    QPushButton,
    QTextEdit,
    QVBoxLayout,
)

from database.database_handler import DatabaseHandler
from importer.file.tcx_file import TcxImportThread
from importer.garmin.garmin import load_sync, save_sync
from ui.dialog_action_bar import DialogActionBar
from ui.themes import THEME
from utils.logger import logger
from utils.translations import _


class iCloudSyncDialog(QDialog):
    """Dialog window for iCloud sync settings."""

    sync_complete_signal = pyqtSignal()

    def __init__(self, file_dir, img_dir, db_handler: DatabaseHandler, parent=None):
        super().__init__(parent)
        self.file_info_label = None
        self.action_bar = None
        self.import_thread = None
        self.status_label = None
        self.log_output = None
        self.progress_bar = None
        self.import_button = None
        self.setWindowTitle("iCloud Sync Settings")
        self.setGeometry(300, 200, 500, 250)

        self.file_dir = file_dir
        self.img_dir = img_dir
        self.db = db_handler

        self.icloud_folder = self.load_saved_folder()
        self.since_date = self.get_last_sync_date()

        self.sync_complete_signal.connect(self.show_sync_complete_message)
        self.initUI()

    def initUI(self):
        layout = QVBoxLayout()

        # ✅ Headline
        headline = QLabel(_("iCloud Synchronisation"))
        headline.setStyleSheet(
            """
                font-size: 16px;
                font-weight: bold;
            """
        )
        layout.addWidget(headline)

        # ✅ Folder selection (One horizontal line)
        folder_layout = QHBoxLayout()
        self.status_label = QLabel(self.get_folder_name())
        label_color = (
            THEME.ACCENT_COLOR_LIGHT if self.icloud_folder else THEME.DISABLED_COLOR
        )
        self.status_label.setStyleSheet(f"color: {label_color};")
        select_button = QPushButton(_("Select iCloud Folder"))
        select_button.clicked.connect(self.select_folder)
        folder_layout.addWidget(self.status_label)
        folder_layout.addStretch()
        folder_layout.addWidget(select_button)
        layout.addLayout(folder_layout)

        self.file_info_label = QLabel(_("No folder selected."))
        layout.addWidget(self.file_info_label)

        self.progress_bar = QProgressBar()
        self.progress_bar.setVisible(False)  # Hidden until import starts
        layout.addWidget(self.progress_bar)

        self.log_output = QTextEdit()
        self.log_output.setReadOnly(True)
        layout.addWidget(self.log_output)
        layout.addStretch(1)

        self.action_bar = DialogActionBar(
            cancel_action=self.close,
            submit_action=self.prepare_import,
            submit_label="Sync",
        )

        layout.addWidget(self.action_bar)

        self.setLayout(layout)

        if self.icloud_folder:
            self.check_and_update_info()

    def get_folder_name(self):
        """Return only the folder name, not full path."""
        return os.path.basename(self.icloud_folder) if self.icloud_folder else _("None")

    @staticmethod
    def get_last_sync_date():
        last_sync_str = load_sync("last_icloud_sync")
        since_date = None
        if last_sync_str:
            try:
                since_date = datetime.datetime.strptime(
                    last_sync_str, "%Y-%m-%d %H:%M:%S"
                )
            except ValueError:
                since_date = None
        return since_date

    def select_folder(self):
        """Open file dialog to select an iCloud folder and save it."""
        folder = QFileDialog.getExistingDirectory(self, "Select iCloud Folder")
        if folder:
            self.icloud_folder = folder
            self.status_label.setText(self.get_folder_name())
            self.save_folder(folder)
            self.check_and_update_info()  # ✅ Auto-check files when folder is selected

    @staticmethod
    def save_folder(folder):
        save_sync("icloud_folder", folder)

    @staticmethod
    def load_saved_folder():
        return load_sync("icloud_folder")

    def check_folder(self, since_date=None):
        """
        Check for new or deleted files in the selected folder.
        :param since_date: datetime or None, only return files modified after this date
        :return: List of found files or None
        """
        if not self.icloud_folder:
            return None

        if since_date and not isinstance(since_date, datetime.datetime):
            return None

        all_files = os.listdir(self.icloud_folder)  # Get all files in the folder
        file_list = []

        for file in all_files:
            file_path = os.path.join(self.icloud_folder, file)

            if os.path.isfile(file_path):  # Ensure it's a file, not a folder
                file_modified_time = datetime.datetime.fromtimestamp(
                    os.path.getmtime(file_path)
                )

                if since_date is None or (
                    isinstance(since_date, datetime.datetime)
                    and file_modified_time > since_date
                ):
                    file_list.append(file)

        return file_list if file_list else None

    def check_and_update_info(self):
        """Check for files and update UI elements accordingly."""
        file_list = self.check_folder(self.since_date)

        if file_list:
            self.file_info_label.setText(
                _("Found {count} new files. Press Sync to import.").format(
                    count=len(file_list)
                )
            )
            self.file_info_label.setStyleSheet(f"color: {THEME.ACCENT_COLOR};")
            self.action_bar.set_submit_enabled(True)
        else:
            self.file_info_label.setText(_("No new files found."))
            self.file_info_label.setStyleSheet(f"color: {THEME.DISABLED_COLOR};")
            self.action_bar.set_submit_enabled(False)

    def prepare_import(self):
        """Copy files to `self.file_dir` and start import."""
        file_list = self.check_folder(self.since_date)
        if not file_list:
            return  # Safety check

        for file in file_list:
            source_path = os.path.join(self.icloud_folder, file)
            dest_path = os.path.join(self.file_dir, file)
            try:
                shutil.copy2(source_path, dest_path)
            except (FileNotFoundError, PermissionError, OSError):
                logger.critical(f"Failed to copy file: {source_path} to {dest_path}")
                return

        self.start_import()  # ✅ Start import after copying

    def start_import(self):
        """Start the import process with a progress bar and log output."""
        self.action_bar.set_submit_enabled(False)
        self.progress_bar.setVisible(True)  # Show progress bar
        self.progress_bar.setValue(0)  # Reset progress

        self.import_thread = TcxImportThread(self.file_dir, self.img_dir, self.db)
        self.import_thread.progress.connect(self.progress_bar.setValue)
        self.import_thread.log.connect(self.log_output.append)
        self.import_thread.finished.connect(self.on_sync_complete)
        self.import_thread.start()

    def on_sync_complete(self):
        """Handle UI updates when import is finished."""
        self.log_output.append(_("iCloud Sync Completed!"))
        save_sync(
            "last_icloud_sync", datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        )
        self.sync_complete_signal.emit()

    def show_sync_complete_message(self):
        """Show a message box when the sync is complete."""
        QMessageBox.information(self, _("Sync Complete"), _("iCloud sync completed!"))
        if self.parent() and hasattr(self.parent(), "refresh_view"):
            self.parent().refresh_view()
        self.accept()
