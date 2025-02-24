import os
import sys

import pytest

from utils.resource_path import resource_path


@pytest.mark.parametrize(
    "is_frozen, _MEIPASS, relative_path, expected",
    [
        (
            False,
            None,
            "test_folder/test_file.txt",
            os.path.abspath("test_folder/test_file.txt"),
        ),
        (
            True,
            "/fake/pyinstaller/temp",
            "test_folder/test_file.txt",
            "/fake/pyinstaller/temp/test_folder/test_file.txt",
        ),
        (False, None, "", os.path.abspath("")),  # Normal mode empty path
        (True, "/fake/path", "", "/fake/path/"),  # PyInstaller mode empty path
        (
            False,
            None,
            "folder with spaces/file.txt",
            os.path.abspath("folder with spaces/file.txt"),
        ),
    ],
)
def test_resource_path(is_frozen, _MEIPASS, relative_path, expected, monkeypatch):
    """Test resource_path in various scenarios."""

    # Simulate PyInstaller mode
    if is_frozen:
        monkeypatch.setattr(sys, "frozen", True, raising=False)
        monkeypatch.setattr(sys, "_MEIPASS", _MEIPASS, raising=False)
    else:
        monkeypatch.delattr(sys, "frozen", raising=False)
        monkeypatch.delattr(sys, "_MEIPASS", raising=False)

    result = resource_path(relative_path)

    # ✅ Normalize paths before comparison to avoid slash mismatches
    assert os.path.normpath(result) == os.path.normpath(expected)
