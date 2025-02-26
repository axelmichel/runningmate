import json
import os
from datetime import datetime, timedelta

import garminconnect
import keyring
from PyQt6.QtCore import QThread, pyqtSignal
from PyQt6.QtWidgets import (
    QDialog,
    QLabel,
    QLineEdit,
    QMessageBox,
    QPushButton,
    QVBoxLayout,
)

from utils.logger import logger
from utils.translations import _

APPDATA_DIR = os.path.expanduser("~/RunningData/appdata")
if not os.path.exists(APPDATA_DIR):
    os.makedirs(APPDATA_DIR)

SYNC_FILE = os.path.join(APPDATA_DIR, "sync.json")

def save_sync(sync_type, sync_value):
    """
    :param sync_type: str, type of sync (e.g., 'last_garmin_sync', 'last_icloud_sync')
    :param sync_value: str
    """
    sync_data = load_full_sync()  # Load existing data
    sync_data[sync_type] = sync_value  # Update the specific sync type

    with open(SYNC_FILE, "w") as f:
        json.dump(sync_data, f)


def load_sync(sync_type):
    """
    Load the sync date for a specific sync type.

    :param sync_type: str, type of sync to retrieve (e.g., 'garmin_sync', 'icloud_sync')
    :return: str or None, last sync date for the given sync type
    """
    sync_data = load_full_sync()
    return sync_data.get(sync_type, None)


def load_full_sync():
    """
    Load all stored sync dates from the JSON file.

    :return: dict, all sync dates (e.g., {'last_garmin_sync': '2025-02-26T10:30:00', 'last_icloud_sync': '2025-02-25T09:00:00'})
    """
    if os.path.exists(SYNC_FILE):
        with open(SYNC_FILE, "r") as f:
            return json.load(f)
    return {}  # Return an empty dictionary if file doesn't exist


def delete_from_sync(sync_type):
    """
    Delete a specific sync type entry from the sync file.

    :param sync_type: str, the type of sync to delete
    """
    sync_data = load_full_sync()
    if sync_type in sync_data:
        del sync_data[sync_type]
        with open(SYNC_FILE, "w") as f:
            json.dump(sync_data, f)


def get_first_activity_date(client):
    try:
        summary = client.get_activity_summary()
        total_activities = summary["totalActivities"]

        if total_activities > 0:
            # Fetch the earliest activity (last in the list)
            activities = client.get_activities(start=total_activities - 1, limit=1)
            if activities:
                first_activity = activities[0]
                first_date = first_activity["startTimeLocal"].split("T")[0]
                return first_date
    except Exception as e:
        logger.warning(f"Failed to retrieve first activity date: {e}")
    return None


def garmin_connect_login():
    username = keyring.get_password("garmin", "username")
    password = keyring.get_password("garmin", "password")

    if not username or not password:
        login_window = LoginWindow()
        if login_window.exec():  # Waits for user input
            username, password = login_window.get_credentials()
            try:
                client = garminconnect.Garmin(username, password)
                client.login()
                keyring.set_password("garmin", "username", username)
                keyring.set_password("garmin", "password", password)
                return client
            except Exception as e:
                QMessageBox.critical(None, _("Login Failed"), f"{e}")
                return None
        else:
            return None

    try:
        client = garminconnect.Garmin(username, password)
        client.login()
        return client
    except Exception:
        keyring.delete_password("garmin", "username")
        keyring.delete_password("garmin", "password")
        return garmin_connect_login()


class LoginWindow(QDialog):
    def __init__(self):
        super().__init__()
        self.setWindowTitle(_("Garmin Connect Login"))
        self.setGeometry(100, 100, 300, 200)

        self.layout = QVBoxLayout()
        self.label = QLabel(_("Enter Garmin Credentials"))
        self.username_input = QLineEdit()
        self.username_input.setPlaceholderText(_("Username"))
        self.password_input = QLineEdit()
        self.password_input.setPlaceholderText(_("Password"))
        self.password_input.setEchoMode(QLineEdit.EchoMode.Password)

        self.login_button = QPushButton(_("Login"))
        self.login_button.clicked.connect(self.store_credentials)

        self.layout.addWidget(self.label)
        self.layout.addWidget(self.username_input)
        self.layout.addWidget(self.password_input)
        self.layout.addWidget(self.login_button)
        self.setLayout(self.layout)

        self.credentials = None

    def store_credentials(self):
        """Store the credentials and close the dialog properly"""
        if self.username_input.text() and self.password_input.text():
            self.credentials = (self.username_input.text(), self.password_input.text())
            self.accept()  # Close the dialog with success
        else:
            QMessageBox.warning(self, _("Input Error"), _("Both fields are required."))

    def get_credentials(self):
        return self.credentials


class SyncGarminThread(QThread):
    progress = pyqtSignal(int)
    log = pyqtSignal(str)
    finished = pyqtSignal()

    def __init__(self, client, start_date, end_date, file_dir, file_format):
        super().__init__()
        self.client = client
        self.start_date = start_date
        self.end_date = end_date
        self.file_dir = file_dir
        self.file_format = file_format

    def run(self):
        current_date = self.start_date
        total_days = (self.end_date - self.start_date).days + 1
        completed = 0
        latest_date = self.start_date

        format_map = {
            "fit": self.client.ActivityDownloadFormat.ORIGINAL,
            "tcx": self.client.ActivityDownloadFormat.TCX,
            "csv": self.client.ActivityDownloadFormat.CSV,
        }

        while current_date <= self.end_date:
            formatted_date = current_date.strftime("%Y-%m-%d")
            try:
                activity_list = self.client.get_activities_by_date(
                    formatted_date, formatted_date
                )
                for activity in activity_list:
                    activity_id = activity["activityId"]
                    filename = os.path.join(
                        self.file_dir,
                        f"garmin_activity_{formatted_date}_{activity_id}.{self.file_format}",
                    )
                    if os.path.exists(filename):
                        self.log.emit(f"Skipping existing file: {filename}")
                        continue
                    activity_data = self.client.download_activity(
                        activity_id, dl_fmt=format_map[self.file_format]
                    )
                    with open(filename, "wb") as f:
                        f.write(activity_data)
                    self.log.emit(
                        f"Downloaded activity {activity_id} for {formatted_date} as {self.file_format.upper()}"
                    )
                    latest_date = max(
                        latest_date, datetime.strptime(formatted_date, "%Y-%m-%d")
                    )
            except Exception as e:
                self.log.emit(f"Failed to retrieve activity for {formatted_date}: {e}")

            completed += 1
            self.progress.emit(int((completed / total_days) * 100))
            current_date += timedelta(days=1)

        save_sync("last_garmin_sync", latest_date.strftime("%Y-%m-%d"))
        self.finished.emit()
