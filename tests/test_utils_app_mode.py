import pytest
from PyQt6.QtGui import QColor, QPalette
from PyQt6.QtWidgets import QApplication

from utils.app_mode import is_color_dark, is_dark_mode


@pytest.fixture
def app():
    """Ensures a QApplication instance exists for testing."""
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    return app


def test_is_color_dark():
    """Test color darkness detection."""
    assert is_color_dark("#000000") is True  # Black is dark
    assert is_color_dark("#FFFFFF") is False  # White is light
    assert is_color_dark("#333333") is True  # Dark gray
    assert is_color_dark("#AAAAAA") is False  # Light gray
    assert is_color_dark("#FF0000") is True  # Red (bright)
    assert is_color_dark("#121212") is True  # Very dark


def test_is_dark_mode_light(app, monkeypatch):
    """Test that is_dark_mode() returns False for light mode."""
    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window, QColor("#FFFFFF"))
    monkeypatch.setattr(app, "palette", lambda: palette)

    assert is_dark_mode() is False


def test_is_dark_mode_dark(app, monkeypatch):
    """Test that is_dark_mode() returns True for dark mode."""
    palette = QPalette()
    palette.setColor(QPalette.ColorRole.Window, QColor("#121212"))
    monkeypatch.setattr(app, "palette", lambda: palette)

    assert is_dark_mode() is True
