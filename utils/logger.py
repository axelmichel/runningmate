import logging
import os
from logging.handlers import RotatingFileHandler

from PyQt6.QtWidgets import QApplication, QMessageBox


class Logger:
    LOG_DIR = os.path.expanduser("~/RunningData/logs")
    LOG_FILE = os.path.join(LOG_DIR, "app.log")
    MAX_FILE_SIZE = 5 * 1024 * 1024  # 5MB per file
    BACKUP_COUNT = 3  # Keep last 3 logs
    DEFAULT_LEVEL = logging.DEBUG if os.getenv("DEBUG_MODE") else logging.WARNING

    _instance = None  # Singleton instance

    def __new__(cls, log_file=None, level=None):
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._initialize(
                log_file or cls.LOG_FILE, level or cls.DEFAULT_LEVEL
            )
        return cls._instance

    def _initialize(self, log_file, level):
        os.makedirs(self.LOG_DIR, exist_ok=True)

        self.logger = logging.getLogger("AppLogger")
        self.logger.setLevel(level)

        console_handler = logging.StreamHandler()
        console_handler.setLevel(logging.INFO)

        file_handler = RotatingFileHandler(
            log_file, maxBytes=self.MAX_FILE_SIZE, backupCount=self.BACKUP_COUNT
        )
        file_handler.setLevel(logging.DEBUG)

        formatter = logging.Formatter("%(asctime)s - %(levelname)s - %(message)s")
        console_handler.setFormatter(formatter)
        file_handler.setFormatter(formatter)

        self.logger.addHandler(console_handler)
        self.logger.addHandler(file_handler)

    def debug(self, message):
        self.logger.debug(message)

    def info(self, message):
        self.logger.info(message)

    def warning(self, message):
        self.logger.warning(message)

    def error(self, message, show_popup=False):
        self.logger.error(message)
        if show_popup:
            self.show_error_popup(message)

    def critical(self, message, show_popup=True):
        self.logger.critical(message)
        if show_popup:
            self.show_error_popup(message)

    @staticmethod
    def show_error_popup(message):
        app = QApplication.instance() or QApplication([])
        msg_box = QMessageBox()
        msg_box.setIcon(QMessageBox.Icon.Critical)
        msg_box.setWindowTitle("Application Error")
        msg_box.setText(message)
        msg_box.exec()  # Display the error window


# Initialize a global logger instance
logger = Logger()
