from PyQt6.QtWidgets import QGraphicsOpacityEffect, QPushButton

from ui.themes import THEME


class OpacityButton(QPushButton):
    def __init__(self, text, parent=None):
        super().__init__(text, parent)

        self.opacity_effect = QGraphicsOpacityEffect()
        self.setGraphicsEffect(self.opacity_effect)
        self.opacity_effect.setOpacity(0.5)  # Default 50% opacity

        self.setCheckable(True)

        self.toggled.connect(self.on_toggle)

    def enterEvent(self, event):
        """Increase opacity when hovered."""
        if self.isEnabled():
            self.opacity_effect.setOpacity(1.0)  # Fully visible
            super().enterEvent(event)

    def leaveEvent(self, event):
        """Decrease opacity when not hovered, unless checked."""
        if self.isEnabled():
            if not self.isChecked():
                self.opacity_effect.setOpacity(0.5)  # Back to 50% transparent
            super().leaveEvent(event)

    def setEnabled(self, enabled: bool) -> None:
        """Override setEnabled to adjust opacity."""
        super().setEnabled(enabled)
        if enabled:
            self.opacity_effect.setOpacity(0.5 if not self.isChecked() else 1.0)
        else:
            self.opacity_effect.setOpacity(0.2)

    def on_toggle(self, checked):
        """Set opacity to 1.0 if checked, else follow normal behavior."""
        if self.isEnabled():
            if checked:
                self.opacity_effect.setOpacity(1.0)  # Fully visible
            else:
                self.opacity_effect.setOpacity(0.5)
