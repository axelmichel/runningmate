import os
import random
import sqlite3
import time

import pytest

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from processing.compute_statistics import generate_activity_title


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
        "title": title,
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

    db.cursor.execute(
        "SELECT * FROM runs WHERE activity_id = ?", (activity_data["id"],)
    )
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

    db.insert_best_performance(
        activity_data["id"], "Running", 5.0, "25:30", "2024-02-20 10:00:00"
    )
    db.cursor.execute(
        "SELECT * FROM best_performances WHERE activity_type = 'Running' AND distance = 5.0"
    )

    results = db.cursor.fetchall()
    assert len(results) == 1
    assert results[0][3] == "25:30"


def test_database_handler_init_no_conn():
    """Test that DatabaseHandler initializes correctly without a provided connection."""
    db_handler = DatabaseHandler(
        db_path=":memory:"
    )  # ✅ Should create a new SQLite connection
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
        "pause": "00:30",
    }

    db.insert_run(run_data)

    db.cursor.execute(
        "SELECT * FROM runs WHERE activity_id = ?", (activity_data["id"],)
    )
    run = db.cursor.fetchone()
    run = dict(run)

    assert run is not None


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


def test_insert_cycling(test_db):
    """Test inserting a cycling activity."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    cycling_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 250,
        "avg_speed": 25.0,
        "avg_power": 200,
        "avg_heart_rate": 155,
        "avg_pace": "04:30",
        "fastest_pace": "03:50",
        "slowest_pace": "05:15",
        "pause": "00:45",
        "map_html": "cycling_map.html",
        "elevation_img": "cycling_elevation.svg",
        "track_img": "cycling_track.svg",
    }

    db.insert_cycling(cycling_data)

    db.cursor.execute(
        "SELECT * FROM cycling WHERE activity_id = ?", (activity_data["id"],)
    )
    cycling = db.cursor.fetchone()
    cycling = dict(cycling)

    assert cycling is not None
    assert cycling["avg_speed"] == cycling_data["avg_speed"]
    assert cycling["avg_power"] == cycling_data["avg_power"]


def test_insert_walk(test_db):
    """Test inserting a walking activity."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    walk_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 50,
        "avg_speed": 5.0,
        "avg_steps": 120,
        "total_steps": 8000,
        "avg_power": 90,
        "avg_heart_rate": 120,
        "avg_pace": "12:00",
        "fastest_pace": "10:30",
        "slowest_pace": "15:00",
        "pause": "00:20",
        "map_html": "walk_map.html",
        "elevation_img": "walk_elevation.svg",
        "track_img": "walk_track.svg",
    }

    db.insert_walking(walk_data)

    db.cursor.execute(
        "SELECT * FROM walking WHERE activity_id = ?", (activity_data["id"],)
    )
    walk = db.cursor.fetchone()
    walk = dict(walk)

    assert walk is not None
    assert walk["avg_speed"] == walk_data["avg_speed"]
    assert walk["total_steps"] == walk_data["total_steps"]


def test_fetch_run_by_activity_id(test_db):
    """Test fetching a run by activity ID."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    run_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 120,
        "avg_speed": 10.5,
        "avg_heart_rate": 160,
        "total_steps": 6000,
        "avg_steps": 210,
        "avg_power": 150,
        "avg_pace": "06:15",
        "fastest_pace": "05:30",
        "slowest_pace": "07:00",
        "pause": "00:50",
        "map_html": "run_map.html",
        "elevation_img": "run_elevation.svg",
        "track_img": "run_track.svg",
    }

    db.insert_run(run_data)

    run = db.fetch_run_by_activity_id(activity_data["id"])
    assert run is not None
    assert run["activity_type"] == activity_data["activity_type"]
    assert run["avg_speed"] == run_data["avg_speed"]


def test_fetch_walk_by_activity_id(test_db):
    """Test fetching a walk by activity ID."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    walk_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 80,
        "avg_speed": 4.5,
        "avg_steps": 150,
        "total_steps": 9000,
        "avg_power": 100,
        "avg_heart_rate": 130,
        "avg_pace": "13:00",
        "fastest_pace": "11:45",
        "slowest_pace": "14:30",
        "pause": "00:15",
        "map_html": "walk_map.html",
        "elevation_img": "walk_elevation.svg",
        "track_img": "walk_track.svg",
    }

    db.insert_walking(walk_data)

    walk = db.fetch_walk_by_activity_id(activity_data["id"])
    assert walk is not None
    assert walk["activity_type"] == activity_data["activity_type"]
    assert walk["avg_steps"] == walk_data["avg_steps"]


def test_fetch_ride_by_activity_id(test_db):
    """Test fetching a cycling ride by activity ID."""
    db = test_db
    activity_data = generate_test_activity(test_db)
    db.insert_activity(activity_data)

    ride_data = {
        "activity_id": activity_data["id"],
        "elevation_gain": 500,
        "avg_speed": 27.0,
        "avg_power": 250,
        "avg_heart_rate": 140,
        "avg_pace": "03:50",
        "fastest_pace": "03:20",
        "slowest_pace": "04:30",
        "pause": "00:30",
        "map_html": "ride_map.html",
        "elevation_img": "ride_elevation.svg",
        "track_img": "ride_track.svg",
    }

    db.insert_cycling(ride_data)

    ride = db.fetch_ride_by_activity_id(activity_data["id"])
    assert ride is not None
    assert ride["activity_type"] == activity_data["activity_type"]
    assert ride["avg_power"] == ride_data["avg_power"]


def test_fetch_all_activities(test_db):
    """Test fetching all activities with pagination."""
    db = test_db

    for _ in range(5):
        activity_data = generate_test_activity(test_db)
        db.insert_activity(activity_data)

    activities = db.fetch_activities(start=0, limit=5)
    assert len(activities) == 5  # ✅ Ensure all activities are retrieved


def test_fetch_all_runs(test_db):
    """Test fetching all runs with pagination."""
    db = test_db

    for _ in range(3):
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

    runs = db.fetch_runs(start=0, limit=3)
    assert len(runs) == 3  # ✅ Ensure all runs are retrieved


def test_fetch_all_rides(test_db):
    """Test fetching all cycling rides with pagination."""
    db = test_db

    ride_entries = []
    for _ in range(5):  # Insert 5 cycling activities
        activity_data = generate_test_activity(test_db)
        db.insert_activity(activity_data)

        ride_data = {
            "activity_id": activity_data["id"],
            "elevation_gain": random.randint(200, 1000),
            "avg_speed": round(random.uniform(20.0, 35.0), 2),
            "avg_power": random.randint(180, 300),
            "avg_heart_rate": random.randint(120, 160),
            "avg_pace": f"{random.randint(3, 5)}:{random.randint(0, 59):02d}",
            "fastest_pace": f"{random.randint(3, 5)}:{random.randint(0, 59):02d}",
            "slowest_pace": f"{random.randint(5, 7)}:{random.randint(0, 59):02d}",
            "pause": "00:30",
            "map_html": "ride_map.html",
            "elevation_img": "ride_elevation.svg",
            "track_img": "ride_track.svg",
        }

        db.insert_cycling(ride_data)
        ride_entries.append(ride_data)

    # ✅ Fetch the rides
    rides = db.fetch_rides(start=0, limit=5)

    # ✅ Verify that we retrieved the correct number of rides
    assert len(rides) == 5

    # ✅ Check that the data matches what we inserted
    for i, ride in enumerate(rides):
        assert ride["activity_id"] == ride_entries[i]["activity_id"]
        assert ride["avg_speed"] == ride_entries[i]["avg_speed"]
        assert ride["avg_pace"] == ride_entries[i]["avg_pace"]
        assert ride["fastest_pace"] == ride_entries[i]["fastest_pace"]
        assert ride["slowest_pace"] == ride_entries[i]["slowest_pace"]


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
    db.cursor.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    )
    tables = db.cursor.fetchall()

    for (table_name,) in tables:
        db.cursor.execute(f"DROP TABLE {table_name};")  # ✅ Drop the table

    db.conn.commit()

    # ✅ Re-run migrations to recreate tables
    apply_migrations(db)

    db.cursor.execute(
        "PRAGMA foreign_keys = ON;"
    )  # ✅ Re-enable foreign key constraints


@pytest.fixture(scope="module", autouse=True)
def cleanup_test_files():
    """Cleanup any test-generated files after tests."""
    yield
    test_files = ["test_media.jpg"]
    for file in test_files:
        if os.path.exists(file):
            os.remove(file)
