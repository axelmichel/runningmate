from PyQt6.QtGui import QPalette
from PyQt6.QtWidgets import QApplication


def is_dark_mode():
    """Detects if the application is in dark mode."""
    app = QApplication.instance()
    if not app:
        return False  # Default to light mode if no app instance

    palette = app.palette()
    bg_color = palette.color(
        QPalette.ColorRole.Window
    ).name()  # Get window background color

    # Check if the background color is dark
    return is_color_dark(bg_color)


def is_color_dark(hex_color):
    """Determines if a given hex color is dark."""
    hex_color = hex_color.lstrip("#")
    r, g, b = int(hex_color[0:2], 16), int(hex_color[2:4], 16), int(hex_color[4:6], 16)

    # Compute brightness using luminance formula
    brightness = (r * 299 + g * 587 + b * 114) / 1000
    return brightness < 128  # Dark if brightness is below threshold
