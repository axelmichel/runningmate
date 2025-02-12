import sys
from PyQt6.QtWidgets import QMenuBar, QMenu, QApplication
from PyQt6.QtGui import QAction, QKeySequence
from translations import _


class MenuBar(QMenuBar):
    def __init__(self, parent=None):
        super().__init__(parent)

        file_menu = QMenu(_("File"), self)
        view_menu = QMenu(_("View"), self)
        tools_menu = QMenu(_("Tools"), self)
        help_menu = QMenu(_("Help"), self)

        self.addMenu(file_menu)
        self.addMenu(view_menu)
        self.addMenu(tools_menu)
        self.addMenu(help_menu)

        is_mac = sys.platform == "darwin"

        import_shortcut = QKeySequence.StandardKey.Open if is_mac else QKeySequence("Ctrl+I")
        settings_shortcut = QKeySequence.StandardKey.Preferences if is_mac else QKeySequence("Ctrl+S")
        export_shortcut = QKeySequence("Ctrl+E")
        quit_shortcut = QKeySequence.StandardKey.Quit if is_mac else QKeySequence("Ctrl+Q")
        search_shortcut = QKeySequence.StandardKey.Find if is_mac else QKeySequence("Ctrl+F")
        new_vault_shortcut = QKeySequence("Ctrl+N")
        about_shortcut = QKeySequence.StandardKey.HelpContents if is_mac else QKeySequence("F1")

        import_action = QAction("Import TCX", self)
        import_action.setShortcut(import_shortcut)
        import_action.triggered.connect(parent.upload_tcx_file)  # Connect to parent function
        file_menu.addAction(import_action)

        settings_action = QAction("Settings", self)
        settings_action.setShortcut(settings_shortcut)
        file_menu.addAction(settings_action)

        export_action = QAction("Export Excel", self)
        export_action.setShortcut(export_shortcut)
        file_menu.addAction(export_action)

        file_menu.addSeparator()

        quit_action = QAction("Quit", self)
        quit_action.setShortcut(quit_shortcut)
        quit_action.setMenuRole(QAction.MenuRole.NoRole)  # Ensure it stays in File menu
        quit_action.triggered.connect(QApplication.instance().quit)  # Force quit all windows
        file_menu.addAction(quit_action)

        search_action = QAction("Search", self)
        search_action.setShortcut(search_shortcut)
        view_menu.addAction(search_action)

        new_vault_action = QAction("New Vault", self)
        new_vault_action.setShortcut(new_vault_shortcut)
        tools_menu.addAction(new_vault_action)

        about_action = QAction("About", self)
        about_action.setShortcut(about_shortcut)
        about_action.setMenuRole(QAction.MenuRole.NoRole)  # Prevent it from being hidden
        help_menu.addAction(about_action)
