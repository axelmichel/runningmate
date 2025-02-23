from PyQt6.QtWidgets import QGraphicsOpacityEffect, QPushButton


class OpacityButton(QPushButton):
    def __init__(self, text, parent=None):
        super().__init__(text, parent)

        # Apply opacity effect
        self.opacity_effect = QGraphicsOpacityEffect()
        self.setGraphicsEffect(self.opacity_effect)
        self.opacity_effect.setOpacity(0.5)  # Default 50% opacity

        # Enable checkable state
        self.setCheckable(True)

        # Connect toggle event to change opacity
        self.toggled.connect(self.on_toggle)

    def enterEvent(self, event):
        """Increase opacity when hovered."""
        self.opacity_effect.setOpacity(1.0)  # Fully visible
        super().enterEvent(event)

    def leaveEvent(self, event):
        """Decrease opacity when not hovered, unless checked."""
        if not self.isChecked():
            self.opacity_effect.setOpacity(0.5)  # Back to 50% transparent
        super().leaveEvent(event)

    def on_toggle(self, checked):
        """Set opacity to 1.0 if checked, else follow normal behavior."""
        if checked:
            self.opacity_effect.setOpacity(1.0)  # Fully visible
        else:
            self.opacity_effect.setOpacity(0.5)  #
