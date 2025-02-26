import sys

from PyQt6.QtGui import QAction, QKeySequence
from PyQt6.QtWidgets import QApplication, QMenu, QMenuBar

from utils.translations import _


class MenuBar(QMenuBar):
    def __init__(self, parent=None):
        super().__init__(parent)

        file_menu = QMenu(_("File"), self)
        view_menu = QMenu(_("View"), self)
        help_menu = QMenu(_("Help"), self)

        self.addMenu(file_menu)
        self.addMenu(view_menu)
        self.addMenu(help_menu)

        is_mac = sys.platform == "darwin"

        import_shortcut = (
            QKeySequence.StandardKey.Open if is_mac else QKeySequence("Ctrl+I")
        )
        settings_shortcut = (
            QKeySequence.StandardKey.Preferences if is_mac else QKeySequence("Ctrl+S")
        )
        quit_shortcut = (
            QKeySequence.StandardKey.Quit if is_mac else QKeySequence("Ctrl+Q")
        )
        search_shortcut = (
            QKeySequence.StandardKey.Find if is_mac else QKeySequence("Ctrl+F")
        )
        online_help_shortcut = (
            QKeySequence.StandardKey.HelpContents if is_mac else QKeySequence("F1")
        )

        import_action = QAction("Import TCX", self)
        import_action.setShortcut(import_shortcut)
        import_action.triggered.connect(parent.upload_tcx_file)
        file_menu.addAction(import_action)

        garmin_action = QAction("Garmin Connect", self)
        garmin_action.triggered.connect(parent.garmin_connect)
        file_menu.addAction(garmin_action)

        icloud_action = QAction("ICloud Sync", self)
        icloud_action.triggered.connect(parent.icloud_sync)
        file_menu.addAction(icloud_action)

        settings_action = QAction("Settings", self)
        settings_action.setShortcut(settings_shortcut)
        file_menu.addAction(settings_action)

        file_menu.addSeparator()

        quit_action = QAction("Quit", self)
        quit_action.setShortcut(quit_shortcut)
        quit_action.setMenuRole(QAction.MenuRole.NoRole)  # Ensure it stays in File menu
        quit_action.triggered.connect(
            QApplication.instance().quit
        )  # Force quit all windows
        file_menu.addAction(quit_action)

        search_action = QAction("Search", self)
        search_action.setShortcut(search_shortcut)
        view_menu.addAction(search_action)

        about_action = QAction("About", self)
        about_action.setMenuRole(
            QAction.MenuRole.NoRole
        )  # Prevent it from being hidden
        about_action.triggered.connect(parent.show_about)
        help_menu.addAction(about_action)

        online_help_action = QAction("Online Help", self)
        online_help_action.setMenuRole(
            QAction.MenuRole.NoRole
        )  # Prevent it from being hidden
        online_help_action.setShortcut(online_help_shortcut)
        online_help_action.triggered.connect(parent.online_help)
        help_menu.addAction(online_help_action)
