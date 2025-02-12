import random
import sqlite3
import os
import time

import pytest
from database.migrations import apply_migrations, get_current_version
from database.database_handler import DatabaseHandler
from processing.compute_statistics import generate_activity_title
from processing.system_settings import ViewMode


@pytest.fixture()
def test_db():
    """Setup an in-memory database with migrations applied for testing."""
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    db = DatabaseHandler(conn=conn)  # ✅ Inject test connection
    apply_migrations(db)  # ✅ Run latest migrations
    yield db  # ✅ Return test database handler
    db.close()  # ✅ Cleanup after test

def generate_test_activity(test_db):
    types = ["Running", "Walking", "Cycling"]
    db = test_db
    activity_id = db.get_next_activity_id()
    distance = round(random.uniform(1.0, 42.2), 2)  # Random distance (1km to 42.2km)
    activity_type = random.choice(list(types))  # Random activity type
    duration = random.randint(600, 14400)  # Random duration (10 min to 4 hours)
    timestamp = int(time.time())  # Current Unix timestamp

    # ✅ Generate title based on activity type & timestamp
    title = generate_activity_title(activity_type, timestamp)

    # ✅ Create test data dictionary
    test_data = {
        "id": activity_id,
        "distance": distance,
        "activity_type": activity_type,
        "duration": duration,
        "date": timestamp,
        "title": title
    }

    return test_data


### --- DatabaseHandler Unit Tests ---

def test_insert_activity(test_db):
    """Test inserting a new activity."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    db.cursor.execute("SELECT * FROM activities WHERE id = ?", (activity_data["id"],))
    activity = db.cursor.fetchone()
    activity = dict(activity)

    assert activity is not None
    assert activity["distance"] == activity_data["distance"]
    assert activity["activity_type"] == activity_data["activity_type"]
    assert activity["title"] == activity_data["title"]


def test_insert_run(test_db):
    """Test inserting a run linked to an activity."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    run_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 100,
        "avg_speed": 10.0,
        "avg_heart_rate": 170,
        "total_steps": 4500,
        "avg_steps": 200,
        "avg_power": 140,
        "avg_pace": "06:00",
        "fastest_pace": "06:00",
        "slowest_pace": "07:30",
        "pause": "00:30",
        "map_html": "map.html",
        "elevation_img": "elevation.svg",
        "track_img": "elevation.svg",
    }

    db.insert_run(run_data)

    db.cursor.execute("SELECT * FROM runs WHERE activity_id = ?", (activity_data["id"],))
    run = db.cursor.fetchone()
    run = dict(run)

    assert run is not None
    assert run["avg_speed"] == run_data["avg_speed"]
    assert run["avg_heart_rate"] == run_data["avg_heart_rate"]


def test_update_comment(test_db):
    """Test updating an activity comment."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    db.update_comment(activity_data["id"], "Great run!")
    comment = db.get_comment(activity_data["id"])

    assert comment == "Great run!"


def test_insert_media(test_db):
    """Test inserting media files."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    db.insert_media(activity_data["id"], "image", "test_image.jpg")
    media_files = db.get_media_files(activity_data["id"])

    assert len(media_files) == 1
    assert media_files[0][1] == "image"
    assert media_files[0][2] == "test_image.jpg"


def test_delete_media(test_db):
    """Test deleting media files."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)
    file_path = "test_media.jpg"

    # Create a dummy file to test deletion
    with open(file_path, "w") as f:
        f.write("test")

    db.insert_media(activity_data["id"], "image", file_path)
    assert os.path.exists(file_path)  # ✅ File should exist before deletion

    db.delete_media(activity_data["id"], file_path)
    assert not os.path.exists(file_path)  # ✅ File should be deleted

    media_files = db.get_media_files(activity_data["id"])
    assert len(media_files) == 0  # ✅ No media should be left


def test_get_runs(test_db):
    """Test retrieving runs for a specific year and month."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    run_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 100,
        "avg_speed": 10.0,
        "avg_heart_rate": 170,
        "total_steps": 4500,
        "avg_steps": 200,
        "avg_power": 140,
        "avg_pace": "06:00",
        "fastest_pace": "06:00",
        "slowest_pace": "07:30",
        "pause": "00:30",
        "map_html": "map.html",
        "elevation_img": "elevation.svg",
        "track_img": "elevation.svg",
    }

    db.insert_run(run_data)

    runs = db.fetch_runs()

    assert len(runs) == 1


def test_best_performance_insert(test_db):
    """Test inserting best performance records."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    db.insert_best_performance(activity_data["id"], "Running", 5.0, "25:30", "2024-02-20 10:00:00")
    db.cursor.execute("SELECT * FROM best_performances WHERE activity_type = 'Running' AND distance = 5.0")

    results = db.cursor.fetchall()
    assert len(results) == 1
    assert results[0][3] == "25:30"


def test_database_handler_init_no_conn():
    """Test that DatabaseHandler initializes correctly without a provided connection."""
    db_handler = DatabaseHandler(db_path=":memory:")  # ✅ Should create a new SQLite connection
    assert db_handler.conn is not None
    db_handler.close()


def test_insert_run_without_images(test_db):
    """Test inserting a run without providing track/elevation images."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    run_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 100,
        "avg_speed": 10.0,
        "avg_heart_rate": 170,
        "total_steps": 4500,
        "avg_steps": 200,
        "avg_power": 140,
        "avg_pace": "06:00",
        "fastest_pace": "06:00",
        "slowest_pace": "07:30",
        "pause": "00:30"
    }

    db.insert_run(run_data)

    db.cursor.execute("SELECT * FROM runs WHERE activity_id = ?", (activity_data["id"],))
    run = db.cursor.fetchone()
    run = dict(run)

    assert run is not None


def test_insert_run_details(test_db):
    """Test inserting run segment details."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    db.insert_run_details(activity_data["id"], 1, 140, 10.5, "05:45", "00:05")

    db.cursor.execute("SELECT * FROM run_details WHERE activity_id = ?", (activity_data["id"],))
    details = db.cursor.fetchone()

    assert details is not None
    assert details[2] == 1  # ✅ Corrected: segment_number
    assert details[3] == 140  # ✅ Corrected: heart_rate
    assert details[4] == 10.5  # ✅ Corrected: speed
    assert details[5] == "05:45"  # ✅ Corrected: pace
    assert details[6] == "00:05"  # ✅ Corrected: pause_time


def test_get_comment(test_db):
    """Test retrieving an activity comment."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    db.update_comment(activity_data["id"], "Morning jog")
    comment = db.get_comment(activity_data["id"])

    assert comment == "Morning jog"  # ✅ Correctly retrieved


def test_get_media_files(test_db):
    """Test retrieving media files for an activity."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    db.insert_media(activity_data["id"], "image", "test_image.jpg")
    db.insert_media(activity_data["id"], "video", "test_video.mp4")

    media_files = db.get_media_files(activity_data["id"])

    assert len(media_files) == 2
    assert media_files[0][1] == "image"
    assert media_files[1][1] == "video"


def test_delete_media_file_not_exist(test_db):
    """Test deleting media that does not exist on disk."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)
    file_path = "non_existent.jpg"

    db.insert_media(activity_data["id"], "image", file_path)
    db.delete_media(activity_data["id"], file_path)

    media_files = db.get_media_files(activity_data["id"])
    assert len(media_files) == 0  # ✅ Ensure media is removed from DB


def test_close_database(test_db):
    """Test closing the database connection."""
    db = test_db
    db.close()
    with pytest.raises(sqlite3.ProgrammingError):  # ✅ Ensure connection is closed
        db.cursor.execute("SELECT 1")


@pytest.fixture(autouse=True)
def reset_database(test_db):
    """Fully resets the database before each test to ensure a clean state."""
    db = test_db

    # ✅ Disable foreign key checks to avoid integrity constraints
    db.cursor.execute("PRAGMA foreign_keys = OFF;")

    # ✅ Drop all tables to completely reset IDs and data
    db.cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
    tables = db.cursor.fetchall()

    for (table_name,) in tables:
        db.cursor.execute(f"DROP TABLE {table_name};")  # ✅ Drop the table

    db.conn.commit()

    # ✅ Re-run migrations to recreate tables
    apply_migrations(db)

    db.cursor.execute("PRAGMA foreign_keys = ON;")  # ✅ Re-enable foreign key constraints


@pytest.fixture(scope="module", autouse=True)
def cleanup_test_files():
    """Cleanup any test-generated files after tests."""
    yield
    test_files = ["test_media.jpg"]
    for file in test_files:
        if os.path.exists(file):
            os.remove(file)

