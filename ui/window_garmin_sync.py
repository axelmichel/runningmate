from datetime import datetime

from PyQt6.QtWidgets import QDialog, QProgressBar, QPushButton, QTextEdit, QVBoxLayout

from database.database_handler import DatabaseHandler
from importer.file.tcx_file import TcxImportThread
from importer.garmin.garmin import (
    SyncGarminThread,
    get_first_activity_date,
    load_sync_date,
)
from utils.translations import _


class GarminSyncWindow(QDialog):
    def __init__(self, garmin_client, file_dir, img_dir, db_handler: DatabaseHandler):
        super().__init__()
        self.import_thread = None
        self.sync_thread = None
        self.start_button = None
        self.layout = None
        self.progress_bar = None
        self.log_output = None
        self.client = garmin_client
        self.file_dir = file_dir
        self.img_dir = img_dir
        self.db = db_handler
        self.init_ui()

    def init_ui(self):
        self.setWindowTitle(_("Garmin Sync"))
        self.setGeometry(100, 100, 400, 300)

        self.layout = QVBoxLayout()
        self.progress_bar = QProgressBar()
        self.log_output = QTextEdit()
        self.log_output.setReadOnly(True)
        self.start_button = QPushButton(_("Start Sync"))
        self.start_button.clicked.connect(self.start_sync)

        self.layout.addWidget(self.progress_bar)
        self.layout.addWidget(self.log_output)
        self.layout.addWidget(self.start_button)
        self.setLayout(self.layout)

    def start_sync(self):
        min_date_str = load_sync_date()
        if not min_date_str:
            min_date_str = get_first_activity_date(self.client)

        start_date = (
            datetime.strptime(min_date_str.split(" ")[0], "%Y-%m-%d")
            if min_date_str
            else datetime(datetime.today().year, 1, 1)
        )
        end_date = datetime.today()
        self.sync_thread = SyncGarminThread(
            self.client, start_date, end_date, self.file_dir, "tcx"
        )
        self.sync_thread.progress.connect(self.progress_bar.setValue)
        self.sync_thread.log.connect(self.log_output.append)
        self.sync_thread.finished.connect(self.on_sync_complete)
        self.sync_thread.start()

    def on_sync_complete(self):
        self.log_output.append(_("Sync Completed!"))  # Log completion
        self.start_import()  # Start import process

    def start_import(self):
        self.import_thread = TcxImportThread(self.file_dir, self.img_dir, self.db)
        self.import_thread.log.connect(self.log_output.append)
        self.import_thread.finished.connect(
            lambda: self.log_output.append(_("Import Completed!"))
        )
        self.import_thread.start()
