import time

import pytest

from processing.activity_info import ActivityInfo


def insert_activity(
    db,
    activity_id,
    activity_type,
    date,
    title,
    distance,
    duration,
    elevation_gain,
    file_id,
):
    """Helper function to insert test data into activities table."""
    db.conn.execute(
        """
        INSERT INTO activities (id, activity_type, date, title, distance, duration, elevation_gain, file_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?);
        """,
        (
            activity_id,
            activity_type,
            date,
            title,
            distance,
            duration,
            elevation_gain,
            file_id,
        ),
    )
    db.conn.commit()


def insert_weather(
    db, activity_id, max_temp, min_temp, precipitation, max_wind_speed, weather_code
):
    """Helper function to insert test data into weather table."""
    db.conn.execute(
        """
        INSERT INTO weather (activity_id, max_temp, min_temp, precipitation, max_wind_speed, weather_code)
        VALUES (?, ?, ?, ?, ?, ?);
        """,
        (activity_id, max_temp, min_temp, precipitation, max_wind_speed, weather_code),
    )
    db.conn.commit()


def insert_media(db, activity_id, media_type, file_path):
    """Helper function to insert test data into media table."""
    db.conn.execute(
        """
        INSERT INTO media (activity_id, media_type, file_path)
        VALUES (?, ?, ?);
        """,
        (activity_id, media_type, file_path),
    )
    db.conn.commit()


@pytest.fixture
def activity_info(test_db):
    """Fixture to create ActivityInfo instance."""
    return ActivityInfo(test_db, "/fake/path")


def test_get_latest_activity_id(test_db, activity_info):
    """Test retrieving the latest activity ID."""
    insert_activity(
        test_db,
        1,
        "Running",
        int(time.time()) - 1000,
        "Morning Run",
        5.0,
        1800,
        50,
        None,
    )
    insert_activity(
        test_db, 2, "Cycling", int(time.time()), "Evening Ride", 20.0, 3600, 100, None
    )
    latest_id = activity_info.get_latest_activity_id()
    assert latest_id == 2  # Latest activity should be the last inserted


def test_get_activity_info(test_db, activity_info):
    """Test retrieving activity details."""
    insert_activity(
        test_db,
        1,
        "Running",
        int(time.time()) - 1000,
        "Morning Run",
        5.0,
        1800,
        50,
        None,
    )
    insert_weather(test_db, 1, 15, 10, 2.3, 20, "Partly Cloudy")
    insert_media(test_db, 1, "image", "/media/photos/run1.jpg")
    activity_details = activity_info.get_activity_info(activity_id=1)
    assert activity_details is not None
    assert activity_details["id"] == 1
    assert activity_details["category"] == "Running"
    assert activity_details["title"] == "Morning Run"
    assert (
        activity_details["weather"]["avg_temp"] == 12.5
    )  # Average of max_temp and min_temp
    assert activity_details["media"]["file_path"] == "/media/photos/run1.jpg"


def test_get_activity_info_no_data(test_db, activity_info):
    """Test handling when no activity data exists."""
    assert activity_info.get_activity_info(activity_id=99) is None
