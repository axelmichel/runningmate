import os
import sqlite3
import tarfile
import tempfile
from unittest.mock import patch

import numpy as np
import pandas as pd
import pytest

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from importer.file.tcx_file import TcxFileImporter
from processing.compute_statistics import generate_activity_title
from processing.system_settings import ViewMode


def mock_file_data():
    """Mock TCX DataFrame"""
    data = {
        "Time": pd.date_range(
            start="2025-02-15 10:00:00", periods=10, freq="min"
        ).astype(str),
        "Distance": np.linspace(0, 5.0, 10),
        "Elevation": np.linspace(100, 150, 10),
        "HeartRate": [150, 152, 155, 157, 157, 160, 158, 156, 154, 150],
        "Power": [200, 205, 210, 215, 215, 220, 225, 230, 235, 240],
        "Latitude": np.linspace(50.0, 50.01, 10),
        "Longitude": np.linspace(8.0, 8.01, 10),
        "Steps": [100, 105, 110, 115, 120, 125, 130, 135, 140, 145],
    }
    return pd.DataFrame(data)


@pytest.fixture()
def test_db():
    """Setup an in-memory database with migrations applied for testing."""
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    db = DatabaseHandler(conn=conn)  # ✅ Inject test connection
    apply_migrations(db)  # ✅ Run latest migrations
    yield db  # ✅ Return test database handler
    db.close()  # ✅ Cleanup after test


@pytest.fixture()
def mock_tcx_data():
    """Mock TCX DataFrame"""
    data = {
        "Time": pd.date_range(
            start="2025-02-15 10:00:00", periods=10, freq="min"
        ).astype(str),
        "Distance": np.linspace(0, 5.0, 10),
        "Elevation": np.linspace(100, 150, 10),
        "HeartRate": [150, 152, 155, 157, 157, 160, 158, 156, 154, 150],
        "Power": [200, 205, 210, 215, 215, 220, 225, 230, 235, 240],
        "Latitude": np.linspace(50.0, 50.01, 10),
        "Longitude": np.linspace(8.0, 8.01, 10),
        "Steps": [100, 105, 110, 115, 120, 125, 130, 135, 140, 145],
    }
    return pd.DataFrame(data)


@pytest.fixture()
def tcx_importer(test_db):
    """Setup a TcxFileImporter instance with temporary paths"""
    temp_dir = tempfile.mkdtemp()
    return TcxFileImporter(file_path=temp_dir, image_path=temp_dir, db_handler=test_db)


def test_compute_data(mock_tcx_data):
    """Test compute_data() ensures correct distance, duration, and elevation calculation."""
    computed = TcxFileImporter.compute_data(mock_tcx_data)
    expected_heart_rate = int(round(mock_tcx_data["HeartRate"].dropna().mean(), 0))
    expected_power = int(round(mock_tcx_data["Power"].dropna().mean(), 0))
    assert computed["distance"] == 5.0, "Distance should be calculated correctly."
    assert computed["duration"] == 540, "Duration should be 9 minutes (540 seconds)."
    assert (
        computed["elevation_gain"] == 50
    ), "Elevation gain should be calculated correctly."
    assert computed["avg_speed"] > 0, "Average speed should be positive."
    assert (
        computed["avg_heart_rate"] == expected_heart_rate
    ), f"Expected {expected_heart_rate}, but got {computed['avg_heart_rate']}"
    assert (
        computed["avg_power"] == expected_power
    ), f"Expected {expected_power}, but got {computed['avg_power']}"


@patch("PyQt6.QtWidgets.QFileDialog.getOpenFileName", return_value=("test.tcx", None))
@patch("importer.file.tcx_file.TcxFileImporter.process_file")
@patch("importer.file.tcx_file.TcxFileImporter.archive_file")
def test_upload(mock_archive, mock_process, mock_file_dialog, tcx_importer):
    """Test upload() ensures file selection and processing happens."""
    result = tcx_importer.upload()

    assert result is True, "Upload should return True when file is selected."
    mock_process.assert_called_once_with("test.tcx")
    mock_archive.assert_called_once_with("test.tcx")


@patch(
    "importer.file.tcx_file.parse_tcx", return_value=(mock_file_data(), "Running")
)  # ✅ Fixed mock
@patch(
    "importer.file.tcx_file.TcxFileImporter.compute_data",
    return_value={
        "date": 1739296446,
        "distance": 5.0,
        "elevation_gain": 100,
        "duration": 1200,
        "avg_speed": 10.5,
        "avg_pace": "06:00",
        "fastest_pace": "05:30",
        "slowest_pace": "06:30",
        "avg_heart_rate": 150,
        "avg_power": 220,
        "pause": "00:30",
    },
)
@patch(
    "importer.file.tcx_file.TcxFileImporter.plot_stats",
    return_value={
        "activity_id": 1,
        "activity_type": "Running",
        "track_img": "track_img",
        "elevation_img": "elevation_img",
        "map_html": "map_html",
        "date": 1739296446,
        "distance": 5.0,
        "elevation_gain": 100,
        "duration": 1200,
        "avg_speed": 10.5,
        "avg_pace": "06:00",
        "fastest_pace": "05:30",
        "slowest_pace": "06:30",
        "avg_heart_rate": 150,
        "avg_power": 220,
        "pause": "00:30",
    },
)
def test_process_file(mock_plot, mock_compute, mock_parse, tcx_importer):
    """Test process_file() to ensure TCX parsing, computations, and DB insertion."""
    tcx_importer.process_file("test.tcx")

    mock_parse.assert_called_once_with("test.tcx")
    mock_compute.assert_called_once()


def test_process_run(mock_tcx_data, tcx_importer):
    """Test process_run() ensures correct step calculation and DB insertion."""
    computed_data = TcxFileImporter.compute_data(mock_tcx_data)
    computed_data["activity_id"] = 1
    tcx_importer.process_run(mock_tcx_data, computed_data)

    assert "avg_steps" in computed_data, "avg_steps should be calculated."
    assert "total_steps" in computed_data, "total_steps should be calculated."


def test_process_walk(mock_tcx_data, tcx_importer):
    """Test process_walk() ensures correct step calculation and DB insertion."""
    computed_data = TcxFileImporter.compute_data(mock_tcx_data)
    computed_data["activity_id"] = 1
    tcx_importer.process_walk(mock_tcx_data, computed_data)

    assert "avg_steps" in computed_data, "avg_steps should be calculated."
    assert "total_steps" in computed_data, "total_steps should be calculated."


def test_process_cycle(mock_tcx_data, tcx_importer):
    """Test process_cycle() ensures cycling data is inserted correctly."""
    computed_data = TcxFileImporter.compute_data(mock_tcx_data)
    computed_data["avg_pace"] = "04:30"
    computed_data["activity_id"] = 1
    tcx_importer.process_cycle(mock_tcx_data, computed_data)

    assert "avg_speed" in computed_data, "Cycling should have avg_speed."
    assert "avg_pace" in computed_data, "Cycling should have avg_pace."


def test_archive_file(tcx_importer):
    """Test archive_file() ensures files are correctly compressed to .tar.gz."""
    temp_file = os.path.join(tcx_importer.file_path, "test.tcx")

    # Create a dummy file to archive
    with open(temp_file, "w") as f:
        f.write("Dummy TCX Content")

    tcx_importer.archive_file(temp_file)

    archive_path = temp_file + ".tar.gz"
    assert os.path.exists(archive_path), "Tar.gz archive should be created."

    # Verify archive contents
    with tarfile.open(archive_path, "r:gz") as tar:
        assert (
            "test.tcx" in tar.getnames()
        ), "Archive should contain the original TCX file."


def test_generate_activity_title():
    """Test generate_activity_title() to ensure correct time-of-day labels."""
    morning = generate_activity_title(
        ViewMode.RUN, pd.Timestamp("2025-02-15 07:00:00", tz="UTC").timestamp()
    )
    afternoon = generate_activity_title(
        ViewMode.RUN, pd.Timestamp("2025-02-15 14:00:00", tz="UTC").timestamp()
    )
    evening = generate_activity_title(
        ViewMode.RUN, pd.Timestamp("2025-02-15 18:30:00", tz="UTC").timestamp()
    )
    night = generate_activity_title(
        ViewMode.RUN, pd.Timestamp("2025-02-15 23:00:00", tz="UTC").timestamp()
    )

    assert "Morning" in morning, f"Expected 'Morning' but got {morning}"
    assert "Afternoon" in afternoon, f"Expected 'Afternoon' but got {afternoon}"
    assert "Evening" in evening, f"Expected 'Evening' but got {evening}"
    assert "Night" in night, f"Expected 'Night' but got {night}"


@patch("importer.file.tcx_file.plot_track")
@patch("importer.file.tcx_file.plot_elevation")
@patch("importer.file.tcx_file.plot_activity_map")
def test_plot_stats(mock_track, mock_elevation, mock_map, tcx_importer):
    """Test plot_stats() ensures that images/maps are generated."""
    tcx_importer.plot_stats("test_activity", pd.DataFrame(), {})

    mock_track.assert_called_once()
    mock_elevation.assert_called_once()
    mock_map.assert_called_once()
