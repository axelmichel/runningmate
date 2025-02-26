from PyQt6.QtWidgets import QHBoxLayout, QPushButton, QWidget

from ui.themes import THEME
from utils.translations import _


class DialogActionBar(QWidget):
    """
    An action bar for dialogs with a Cancel and a Submit button.

    Parameters:
    - cancel_action (callable): A function to connect to the Cancel button's click event.
    - submit_action (callable): A function to connect to the Submit button's click event.
    - submit_label (str): The label for the Submit button (default: 'Save').

    Returns:
    - QWidget: A QWidget containing a horizontal layout with Cancel and Submit buttons.
    """

    def __init__(self, cancel_action, submit_action, submit_label="Save", parent=None):
        super().__init__(parent)

        self.submit_btn = None
        self.submit_action = submit_action
        self.cancel_action = cancel_action
        self.submit_label = submit_label

        self.init_ui()

    def init_ui(self):
        # Create Cancel button
        cancel_btn = self.get_cancel_button()

        # Create Submit button
        self.submit_btn = self.get_submit_button()

        # Layout to organize the buttons horizontally
        button_layout = QHBoxLayout()
        button_layout.setContentsMargins(0, 10, 0, 0)
        button_layout.addStretch(1)  # Adds stretchable space before the buttons
        button_layout.addWidget(cancel_btn)
        button_layout.addWidget(self.submit_btn)

        self.setLayout(button_layout)

    def get_submit_button(self):
        """
        Create and return the Submit button with the appropriate label and action.

        :return: QPushButton, Submit button with a connected action.
        """
        submit_btn = QPushButton(_(self.submit_label))
        submit_btn.setFixedWidth(100)
        submit_btn.setStyleSheet(
            f"background-color: {THEME.MAIN_COLOR_LIGHT}; padding: 8px; border-radius: 5px;"
        )
        submit_btn.clicked.connect(self.submit_action)
        return submit_btn

    def set_submit_enabled(self, enabled: bool):
        """
        Enables or disables the Submit button.

        :param enabled: bool, True to enable the Submit button, False to disable it.
        """
        if self.submit_btn:
            self.submit_btn.setEnabled(enabled)

    def get_cancel_button(self):
        """
        Create and return the Cancel button with its respective action.

        :return: QPushButton, Cancel button with a connected action.
        """
        cancel_btn = QPushButton(_("Cancel"))
        cancel_btn.setFixedWidth(100)
        cancel_btn.setStyleSheet(
            f"background-color: {THEME.SYSTEM_BUTTON}; padding: 8px; border-radius: 5px;"
        )
        cancel_btn.clicked.connect(self.cancel_action)
        return cancel_btn
