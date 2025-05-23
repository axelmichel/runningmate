from PyQt6.QtCore import pyqtSignal
from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import QVBoxLayout, QWidget

from ui.opacity_button import OpacityButton
from ui.themes import THEME
from utils.app_mode import is_dark_mode
from utils.resource_path import resource_path
from utils.translations import _


class Sidebar(QWidget):
    """
    Custom Sidebar for Navigation & Tool Buttons (Top/Bottom).

    Signals:
        action_triggered (str): Emitted when a button is clicked.
    """

    action_triggered = pyqtSignal(object)  # Signal emitted when a button is clicked

    def __init__(self, actions, parent=None):
        """
        Initializes a customizable sidebar with buttons.

        :param actions: dict, button mapping in the form:
            {
                "identifier": ("icon_path", "Text"),
                "another_action": ("icon_path", "Another Text"),
            }
        :param parent: QWidget, parent widget (optional)
        """
        super().__init__(parent)

        self.button_states = None
        self.parent_widget = parent
        self.layout = QVBoxLayout(self)
        self.layout.setContentsMargins(0, 0, 0, 0)
        self.layout.setSpacing(10)

        self.icon_folder = "light" if is_dark_mode() else "dark"

        self.actions = {
            key: (resource_path(f"icons/{self.icon_folder}/{icon}"), _(text))
            for key, (icon, text) in actions.items()
        }

        self.buttons = {}
        self.button_states = {}
        for action, (icon_path, text) in self.actions.items():
            button = OpacityButton(f"  {text}")
            button.setCheckable(True)
            button.setAutoExclusive(True)
            button.setIcon(QIcon(icon_path))
            button.clicked.connect(lambda _, a=action: self.set_active_action(a))

            button.setStyleSheet(self.get_button_style())

            self.buttons[action] = button
            self.button_states[action] = True
            self.layout.addWidget(button)

        self.update_button_labels()

    def set_active_action(self, action):
        """
        Sets the active action and emits a signal.

        :param action: str, the selected action
        """
        if action in self.actions:
            self.buttons[action].setChecked(True)
            self.action_triggered.emit(action)

    def set_button_enabled(self, action, enabled):
        """
        Enables or disables a specific button.

        :param action: str, the button identifier.
        :param enabled: bool, whether the button should be enabled or disabled.
        """
        if action in self.buttons:
            self.buttons[action].setEnabled(enabled)
            self.button_states[action] = enabled

    def update_buttons_state(self, states):
        """
        Updates multiple buttons' enabled state based on a dictionary.

        :param states: dict, mapping of action names to boolean enabled states.
            Example: {"identifier": True, "another_action": False}
        """
        for action, enabled in states.items():
            self.set_button_enabled(action, enabled)

    def update_button_labels(self):
        """
        Dynamically updates button labels based on the left panel's width.
        - If the panel is **small**, only icons are shown.
        - If the panel is **wide**, icons + text are shown.
        """
        if self.parent_widget:
            panel_width = self.parent_widget.width()
            show_text = panel_width > 100

            for action, (__, text) in self.actions.items():
                self.buttons[action].setText(f"  {text}" if show_text else "")

    def resizeEvent(self, event):
        """Handles window resizing to update button labels dynamically."""
        self.update_button_labels()
        super().resizeEvent(event)

    def get_button_style(self):
        """
        Returns custom stylesheet for the navigation buttons.

        :return: str, CSS stylesheet for button appearance
        """
        return f"""
            QPushButton {{
                background-color: transparent;
                color: {THEME.NAV_TEXT};
                padding: 8px;
                text-align: left;
                border-radius: 5px;
            }}
        """
