import logging
import os
from unittest.mock import patch

import pytest
from PyQt6.QtWidgets import QApplication, QMessageBox

from utils.logger import Logger


@pytest.fixture(scope="session", autouse=True)
def mock_qt_application():
    """Ensure QApplication instance exists before running UI-related tests."""
    app = QApplication.instance()
    if not app:
        app = QApplication([])  # ✅ Create a dummy Qt application if none exists
    yield app


@pytest.fixture(scope="module", autouse=True)
def mock_qmessagebox():
    """Globally mock QMessageBox to prevent actual popups in tests."""
    with patch.object(QMessageBox, "exec", return_value=None) as mock_exec:
        yield mock_exec  # ✅ Ensure `exec()` does nothing


@pytest.fixture
def temp_logger():
    """Create a fresh logger instance for testing."""
    log_file = "test_app.log"

    Logger._instance = None
    logger = Logger(log_file=log_file, level=logging.DEBUG)

    if not logger.logger.hasHandlers():
        raise RuntimeError("Logger has no handlers after initialization!")

    yield logger  # Return the new logger instance

    if os.path.exists(log_file):
        os.remove(log_file)


@pytest.fixture(autouse=True)
def cleanup_test_logs():
    """Cleanup test logs after tests are done."""
    yield
    if os.path.exists("test_app.log"):
        os.remove("test_app.log")


def test_log_file_creation(temp_logger):
    """Ensure log file is created and writable."""

    print(f"Logger Handlers: {temp_logger.logger.handlers}")

    temp_logger.warning("Testing log file creation")  # Log a test message

    assert temp_logger.logger.handlers, "Logger should have at least one handler"

    file_handler = next(
        (h for h in temp_logger.logger.handlers if isinstance(h, logging.FileHandler)),
        None,
    )

    assert file_handler is not None, "Logger should have a file handler"

    file_handler.flush()
    file_handler.close()

    log_file = file_handler.baseFilename
    assert os.path.exists(log_file), f"Log file {log_file} should be created"


def test_logging_levels(temp_logger):
    """Ensure messages are logged at the correct level."""
    with (
        patch.object(temp_logger.logger, "debug") as mock_debug,
        patch.object(temp_logger.logger, "info") as mock_info,
        patch.object(temp_logger.logger, "warning") as mock_warning,
        patch.object(temp_logger.logger, "error") as mock_error,
        patch.object(temp_logger.logger, "critical") as mock_critical,
    ):
        temp_logger.debug("Debug message")
        temp_logger.info("Info message")
        temp_logger.warning("Warning message")
        temp_logger.error("Error message")
        temp_logger.critical("Critical message")

        mock_debug.assert_called_once_with("Debug message")
        mock_info.assert_called_once_with("Info message")
        mock_warning.assert_called_once_with("Warning message")
        mock_error.assert_called_once_with("Error message")
        mock_critical.assert_called_once_with("Critical message")


def test_prevent_duplicate_handlers():
    """Ensure duplicate handlers are not added to the logger."""
    logger1 = Logger()
    initial_handler_count = len(logger1.logger.handlers)

    logger2 = Logger()
    assert (
        len(logger2.logger.handlers) == initial_handler_count
    ), "Logger should not add duplicate handlers"


@patch.object(QMessageBox, "exec", return_value=None)
def test_error_popup(mock_exec, temp_logger):
    """Ensure error popup is displayed exactly once when enabled."""
    mock_exec.reset_mock()  # ✅ Reset call count before test

    temp_logger.error("This is an error!", show_popup=True)

    assert mock_exec.call_count == 1, f"Expected 1 call, but got {mock_exec.call_count}"


@patch.object(QMessageBox, "exec", return_value=None)
def test_critical_error_popup(mock_exec, temp_logger):
    """Ensure critical error popup is displayed exactly once when enabled."""
    mock_exec.reset_mock()  # ✅ Reset call count before test

    temp_logger.critical("Critical failure!")

    assert mock_exec.call_count == 1, f"Expected 1 call, but got {mock_exec.call_count}"
