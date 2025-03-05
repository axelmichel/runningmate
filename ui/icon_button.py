from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import QGraphicsOpacityEffect, QPushButton

from utils.app_mode import is_dark_mode
from utils.resource_path import resource_path


class IconButton(QPushButton):
    def __init__(self, icon, parent=None):
        super().__init__(parent=parent)
        icon_folder = "light" if is_dark_mode() else "dark"
        icon_path = resource_path(f"icons/{icon_folder}/{icon}")
        self.opacity_effect = QGraphicsOpacityEffect()
        self.setGraphicsEffect(self.opacity_effect)
        self.opacity_effect.setOpacity(1)
        self.setIcon(QIcon(icon_path))
        self.setStyleSheet(get_icon_button_style())


    def setEnabled(self, enabled: bool) -> None:
        """Override setEnabled to adjust opacity."""
        super().setEnabled(enabled)
        if enabled:
            self.opacity_effect.setOpacity(1)
        else:
            self.opacity_effect.setOpacity(0.2)



def get_icon_button_style():
    """
    Returns custom stylesheet for the navigation buttons.

    :return: str, CSS stylesheet for button appearance
    """
    return """
        QPushButton {
            border-radius: 5px;
            border: 1px solid #666;
            padding: 5px;
            background-color: rgba(0, 0, 0, 0);
        }
    """
