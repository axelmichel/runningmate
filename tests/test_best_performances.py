import random
import sqlite3
import time

import pytest

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from processing.best_performances import BestSegmentFinder
from processing.system_settings import ViewMode


def insert_best_performance(
    db, activity_id, distance, best_time, date_time, activity_type
):
    """Helper function to insert test data into best_performances table."""
    db.conn.execute(
        """
        INSERT INTO best_performances (activity_id, distance, best_time, date_time, activity_type)
        VALUES (?, ?, ?, ?, ?);
        """,
        (activity_id, distance, best_time, date_time, activity_type),
    )
    db.conn.commit()


def generate_test_activity(test_db, atype=None):
    types = ["Running", "Walking", "Cycling"]
    db = test_db
    distance = round(random.uniform(1.0, 42.2), 2)  # Random distance (1km to 42.2km)
    activity_type = random.choice(list(types)) if atype is None else atype
    duration = random.randint(600, 14400)  # Random duration (10 min to 4 hours)
    timestamp = int(time.time())  # Current Unix timestamp
    title = "test"

    db.insert_activity(
        {
            "distance": distance,
            "activity_type": activity_type,
            "duration": duration,
            "date": timestamp,
            "title": title,
        }
    )

    cursor = db.cursor
    cursor.execute("SELECT * FROM activities ORDER BY id DESC LIMIT 1")
    latest_entry = cursor.fetchone()
    latest_entry_dict = dict(latest_entry)
    print(latest_entry_dict)
    return latest_entry_dict


@pytest.fixture()
def test_db():
    """Setup an in-memory database with migrations applied for testing."""
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    db = DatabaseHandler(conn=conn)  # ✅ Inject test connection
    apply_migrations(db)  # ✅ Run latest migrations
    yield db  # ✅ Return test database handler
    db.close()  # ✅ Cleanup after test


@pytest.fixture
def best_segment_finder(test_db):
    """Fixture to create BestSegmentFinder instance."""
    return BestSegmentFinder(test_db)


def test_get_best_performance_running(test_db, best_segment_finder):
    """Test retrieving the best performances for running."""
    activity_type = ViewMode.RUN
    print("Running test_get_best_performance_running")
    for _activity_id in range(1, 6):
        generate_test_activity(test_db, activity_type)

    # Insert mock data
    insert_best_performance(test_db, 1, 5, 4.5, "2025-02-23 10:00:00", activity_type)
    insert_best_performance(test_db, 2, 5, 4.2, "2025-02-24 11:00:00", activity_type)
    insert_best_performance(test_db, 3, 5, 4.8, "2025-02-25 12:00:00", activity_type)
    insert_best_performance(test_db, 4, 10, 4.7, "2025-02-26 13:00:00", activity_type)
    insert_best_performance(test_db, 5, 10, 4.3, "2025-02-27 14:00:00", activity_type)

    best_performances = best_segment_finder.get_best_performance(activity_type)

    assert best_performances is not None
    # Check the structure of the output dictionary
    assert "5K" in best_performances
    assert "10K" in best_performances

    # Validate 5K performances
    assert len(best_performances["5K"]) == 3
    assert best_performances["5K"][0]["seg_avg_pace"] == 4.2  # Best time
    assert best_performances["5K"][1]["seg_avg_pace"] == 4.5  # Second best
    assert best_performances["5K"][2]["seg_avg_pace"] == 4.8  # Third best

    # Validate 10K performances
    assert len(best_performances["10K"]) == 2
    assert best_performances["10K"][0]["seg_avg_pace"] == 4.3  # Best time
    assert best_performances["10K"][1]["seg_avg_pace"] == 4.7  # Second best


def test_get_best_performance_cycling(test_db, best_segment_finder):
    """Test retrieving the best performances for cycling."""
    activity_type = ViewMode.CYCLE
    for _ in range(1, 6):
        generate_test_activity(test_db, activity_type)

    # Insert mock data
    insert_best_performance(test_db, 1, 25, 3.2, "2025-02-23 09:00:00", activity_type)
    insert_best_performance(test_db, 2, 25, 3.1, "2025-02-24 10:00:00", activity_type)
    insert_best_performance(test_db, 3, 25, 3.4, "2025-02-25 11:00:00", activity_type)
    insert_best_performance(test_db, 4, 50, 3.8, "2025-02-26 12:00:00", activity_type)

    best_performances = best_segment_finder.get_best_performance(activity_type)

    assert best_performances is not None
    assert "25K" in best_performances
    assert "50K" in best_performances

    assert len(best_performances["25K"]) == 3
    assert best_performances["25K"][0]["seg_avg_pace"] == 3.1
    assert best_performances["25K"][1]["seg_avg_pace"] == 3.2
    assert best_performances["25K"][2]["seg_avg_pace"] == 3.4

    assert len(best_performances["50K"]) == 1
    assert best_performances["50K"][0]["seg_avg_pace"] == 3.8


def test_get_best_performance_walking(test_db, best_segment_finder):
    """Test retrieving the best performances for walking."""
    activity_type = ViewMode.WALK
    for _ in range(1, 6):
        generate_test_activity(test_db, activity_type)

    # Insert mock data
    insert_best_performance(test_db, 1, 5, 5.5, "2025-02-23 08:00:00", activity_type)
    insert_best_performance(test_db, 2, 5, 5.2, "2025-02-24 09:00:00", activity_type)
    insert_best_performance(test_db, 3, 5, 5.8, "2025-02-25 10:00:00", activity_type)
    insert_best_performance(test_db, 4, 10, 5.1, "2025-02-26 11:00:00", activity_type)

    best_performances = best_segment_finder.get_best_performance(activity_type)

    assert best_performances is not None
    assert "5K" in best_performances
    assert "10K" in best_performances

    assert len(best_performances["5K"]) == 3
    assert best_performances["5K"][0]["seg_avg_pace"] == 5.2
    assert best_performances["5K"][1]["seg_avg_pace"] == 5.5
    assert best_performances["5K"][2]["seg_avg_pace"] == 5.8

    assert len(best_performances["10K"]) == 1
    assert best_performances["10K"][0]["seg_avg_pace"] == 5.1


def test_get_best_performance_no_data(test_db, best_segment_finder):
    """Test retrieving the best performances when no data exists."""
    activity_type = ViewMode.RUN
    best_performances = best_segment_finder.get_best_performance(activity_type)
    assert best_performances is None  # Expecting None since no data is available


def test_get_best_segments_run(test_db, best_segment_finder):
    """Test best segment retrieval for a running activity."""
    activity_data = generate_test_activity(test_db)
    activity_id = activity_data["id"]

    sample_data = [
        (activity_id, 0, 1.001, "2025-02-23 13:54:33", "2025-02-23 13:56:33", 5.2),
        (activity_id, 1, 1.001, "2025-02-23 13:56:33", "2025-02-23 13:58:33", 4.9),
        (activity_id, 2, 1.002, "2025-02-23 13:58:33", "2025-02-23 14:03:33", 5.0),
        (activity_id, 3, 0.7, "2025-02-23 14:10:33", "2025-02-23 14:12:33", 3.9),
    ]

    test_db.conn.executemany(
        """
        INSERT INTO activity_details (activity_id, segment_id, seg_distance, seg_time_start, seg_time_end, seg_avg_pace)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        sample_data,
    )

    test_db.conn.commit()  # ✅ Save data in the in-memory database

    # ✅ Fetch best segments using the real database
    best_segments = best_segment_finder.get_best_segments(
        activity_id=int(activity_id), activity_type=ViewMode.RUN
    )

    # ✅ Assert correct structure and values
    assert best_segments is not None
    assert "1K" in best_segments
    assert "5K" not in best_segments  # No 5K distance in sample data
    assert best_segments["1K"]["seg_avg_pace"] == 4.9  # Best 1K pace


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
