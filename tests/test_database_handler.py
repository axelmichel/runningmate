import sqlite3
import os
import pytest
from database.migrations import apply_migrations, get_current_version
from database.database_handler import DatabaseHandler


@pytest.fixture()
def test_db():
    """Setup an in-memory database with migrations applied for testing."""
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    db = DatabaseHandler(conn=conn)  # ✅ Inject test connection
    apply_migrations(db)  # ✅ Run latest migrations
    yield db  # ✅ Return test database handler
    db.close()  # ✅ Cleanup after test


### --- DatabaseHandler Unit Tests ---

def test_insert_activity(test_db):
    """Test inserting a new activity."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    db.cursor.execute("SELECT * FROM activities WHERE id = ?", (activity_id,))
    activity = db.cursor.fetchone()

    assert activity is not None
    assert activity[1] == "2024-02-20 10:00:00"
    assert activity[2] == 5.0
    assert activity[3] == "Running"


def test_insert_run(test_db):
    """Test inserting a run linked to an activity."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    run_data = (
    "2024-02-20", 2, 2024, "10:00:00", 5.0, "30:00", 100, 10.0, 170, 4500, 200, 140, "06:00", "05:30", "07:00", "00:30",
    activity_id)
    db.insert_run(run_data, "track.png", "elevation.svg", "map.html")

    db.cursor.execute("SELECT * FROM runs WHERE activity = ?", (activity_id,))
    run = db.cursor.fetchone()

    assert run is not None
    assert run[1] == "2024-02-20"
    assert run[4] == "10:00:00"
    assert run[5] == 5.0


def test_update_comment(test_db):
    """Test updating an activity comment."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    db.update_comment(activity_id, "Great run!")
    comment = db.get_comment(activity_id)

    assert comment == "Great run!"


def test_insert_media(test_db):
    """Test inserting media files."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    db.insert_media(activity_id, "image", "test_image.jpg")
    media_files = db.get_media_files(activity_id)

    assert len(media_files) == 1
    assert media_files[0][1] == "image"
    assert media_files[0][2] == "test_image.jpg"


def test_delete_media(test_db):
    """Test deleting media files."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")
    file_path = "test_media.jpg"

    # Create a dummy file to test deletion
    with open(file_path, "w") as f:
        f.write("test")

    db.insert_media(activity_id, "image", file_path)
    assert os.path.exists(file_path)  # ✅ File should exist before deletion

    db.delete_media(activity_id, file_path)
    assert not os.path.exists(file_path)  # ✅ File should be deleted

    media_files = db.get_media_files(activity_id)
    assert len(media_files) == 0  # ✅ No media should be left


def test_get_runs(test_db):
    """Test retrieving runs for a specific year and month."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    run_data = (
    "2024-02-20", 2, 2024, "10:00:00", 5.0, "30:00", 100, 10.0, 170, 4500, 200, 140, "06:00", "05:30", "07:00", "00:30",
    activity_id)
    db.insert_run(run_data, "track.png", "elevation.svg", "map.html")

    runs = db.get_runs(2024, 2)

    assert len(runs) == 1
    assert runs[0][1] == "2024-02-20"


def test_best_performance_insert(test_db):
    """Test inserting best performance records."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    db.insert_best_performance(activity_id, "Running", 5.0, "25:30", "2024-02-20 10:00:00")
    db.cursor.execute("SELECT * FROM best_performances WHERE activity_type = 'Running' AND distance = 5.0")

    results = db.cursor.fetchall()
    assert len(results) == 1
    assert results[0][3] == "25:30"


def test_get_years(test_db):
    """Test fetching distinct years."""
    db = test_db
    db.cursor.executemany("""
        INSERT INTO runs (date, month, year, start_time, distance, total_time, elevation_gain, avg_speed, activity) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, [
        ("2023-01-01", "01", "2023", "08:00", 10.0, "00:50:00", 100, 12.0, "Running"),
        ("2024-02-15", "02", "2024", "09:30", 12.0, "01:00:00", 150, 13.5, "Running"),
        ("2024-03-20", "03", "2024", "07:45", 8.5, "00:45:00", 80, 11.0, "Running")
    ])
    db.conn.commit()

    years = db.get_years()
    assert years == ["2024", "2023"]  # ✅ Ensures descending order


def test_get_months(test_db):
    """Test fetching distinct months for a given year."""
    db = test_db
    db.cursor.executemany("""
        INSERT INTO runs (date, year, month, start_time, distance, total_time, elevation_gain, avg_speed, activity) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, [
        ("2024-01-01", "2024", "01", "08:00", 10.0, "00:50:00", 100, 12.0, "Running"),
        ("2024-02-10", "2024", "02", "09:00", 12.5, "01:10:00", 120, 13.2, "Running"),
        ("2024-02-20", "2024", "02", "07:45", 8.5, "00:45:00", 80, 11.0, "Running")
    ])
    db.conn.commit()

    months = db.get_months("2024")
    assert months == ["02", "01"] # ✅ Ensures descending order

def test_database_handler_init_no_conn():
    """Test that DatabaseHandler initializes correctly without a provided connection."""
    db_handler = DatabaseHandler(db_path=":memory:")  # ✅ Should create a new SQLite connection
    assert db_handler.conn is not None
    db_handler.close()


def test_insert_run_without_images(test_db):
    """Test inserting a run without providing track/elevation images."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    run_data = (
        "2024-02-20", "02", "2024", "10:00:00", 5.0, "30:00", 100, 10.0, 170, 4500, 200, 140, "06:00",
        "05:30", "07:00", "00:30", activity_id
    )
    db.insert_run(run_data, None, None, None)  # ✅ Insert without images

    db.cursor.execute("SELECT * FROM runs WHERE activity = ?", (activity_id,))
    run = db.cursor.fetchone()

    assert run is not None
    assert run[1] == "2024-02-20"
    assert run[5] == 5.0  # ✅ Ensure distance is correctly inserted


def test_insert_run_details(test_db):
    """Test inserting run segment details."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    db.insert_run_details(activity_id, 1, 140, 10.5, "05:45", "00:05")



    db.cursor.execute("SELECT * FROM run_details WHERE activity_id = ?", (activity_id,))
    details = db.cursor.fetchone()

    assert details is not None
    assert details[2] == 1  # ✅ Corrected: segment_number
    assert details[3] == 140  # ✅ Corrected: heart_rate
    assert details[4] == 10.5  # ✅ Corrected: speed
    assert details[5] == "05:45"  # ✅ Corrected: pace
    assert details[6] == "00:05"  # ✅ Corrected: pause_time


def test_get_run_by_id(test_db):
    """Test retrieving a run by ID."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    run_data = (
        "2024-02-20", "02", "2024", "10:00:00", 5.0, "30:00", 100, 10.0, 170, 4500, 200, 140, "06:00",
        "05:30", "07:00", "00:30", activity_id
    )
    db.insert_run(run_data, "track.png", "elevation.svg", "map.html")

    db.cursor.execute("SELECT id FROM runs WHERE activity = ?", (activity_id,))
    run_id = db.cursor.fetchone()[0]

    run = db.get_run_by_id(run_id)
    assert run is not None
    assert run[1] == "2024-02-20"
    assert run[5] == 5.0  # ✅ Correct distance


def test_get_comment(test_db):
    """Test retrieving an activity comment."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    db.update_comment(activity_id, "Morning jog")
    comment = db.get_comment(activity_id)

    assert comment == "Morning jog"  # ✅ Correctly retrieved


def test_get_media_files(test_db):
    """Test retrieving media files for an activity."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")

    db.insert_media(activity_id, "image", "test_image.jpg")
    db.insert_media(activity_id, "video", "test_video.mp4")

    media_files = db.get_media_files(activity_id)

    assert len(media_files) == 2
    assert media_files[0][1] == "image"
    assert media_files[1][1] == "video"


def test_delete_media_file_not_exist(test_db):
    """Test deleting media that does not exist on disk."""
    db = test_db
    activity_id = db.insert_activity("2024-02-20 10:00:00", 5.0, "Running")
    file_path = "non_existent.jpg"

    db.insert_media(activity_id, "image", file_path)
    db.delete_media(activity_id, file_path)

    media_files = db.get_media_files(activity_id)
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

