import os
import sqlite3
from datetime import datetime

import pytest

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from processing.plot_heatmap import PlotHeatmap


@pytest.fixture()
def test_db():
    """Setup an in-memory database with migrations applied for testing."""
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    db = DatabaseHandler(conn=conn)
    apply_migrations(db)
    yield db
    db.close()


@pytest.fixture
def plot_heatmap(test_db):
    """Fixture to create PlotHeatmap instance."""
    return PlotHeatmap(test_db)


def insert_activity(db, activity_id, activity_type, date, duration):
    """Helper function to insert test data into activities table."""
    db.conn.execute(
        """
        INSERT INTO activities (id, activity_type, date, duration)
        VALUES (?, ?, ?, ?);
        """,
        (activity_id, activity_type, date, duration),
    )
    db.conn.commit()


def test_get_heatmap_creates_image(test_db, plot_heatmap):
    """Test if heatmap generation creates an image file."""
    activity_type = "Running"
    end_date = datetime.now()
    insert_activity(test_db, 1, activity_type, int(end_date.timestamp()) - 100000, 1800)
    insert_activity(test_db, 2, activity_type, int(end_date.timestamp()) - 50000, 3600)

    heatmap_path = plot_heatmap.get_heatmap(activity_type, end_date, redraw=True)
    assert os.path.exists(heatmap_path)


def test_get_heatmap_returns_cached_image(test_db, plot_heatmap):
    """Test if cached heatmap image is used when redraw=False."""
    activity_type = "Running"
    end_date = datetime.now()
    cached_path = os.path.join(
        os.path.expanduser("~/RunningData/temp"), f"heatmap_{activity_type}.png"
    )
    open(cached_path, "w").close()  # Create a fake cached file

    heatmap_path = plot_heatmap.get_heatmap(activity_type, end_date, redraw=False)
    assert heatmap_path == cached_path
    os.remove(cached_path)  # Cleanup


def test_get_heatmap_handles_no_data(test_db, plot_heatmap):
    """Test if heatmap generation handles an empty dataset correctly."""
    activity_type = "Running"
    end_date = datetime.now()
    heatmap_path = plot_heatmap.get_heatmap(activity_type, end_date, redraw=True)
    assert os.path.exists(heatmap_path)


def test_clear_heatmaps(plot_heatmap):
    """Test if heatmap cache clearing removes all heatmap images."""
    fake_file = os.path.join(
        os.path.expanduser("~/RunningData/temp"), "heatmap_test.png"
    )
    open(fake_file, "w").close()
    plot_heatmap.clear_heatmaps()
    assert not os.path.exists(fake_file)
