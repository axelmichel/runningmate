import os
import sys


def resource_path(relative_path):
    """Get the correct path inside a PyInstaller bundle."""
    if hasattr(sys, "frozen"):
        base_path = sys._MEIPASS  # PyInstaller temp folder
    else:
        base_path = os.path.abspath(".")  # Normal dev mode
    return os.path.join(base_path, relative_path)
