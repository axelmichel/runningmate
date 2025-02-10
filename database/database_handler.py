import sqlite3
import os

from database.migrations import apply_migrations

DB_PATH = os.path.expanduser("~/RunningData/running_data.db")


def connect():
    """Initialize the database with the required schema."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    return conn, cursor


def initialize_database():
    apply_migrations()


def insert_run(data, track_img, elevation_img, map_html):
    conn, cursor = connect()

    cursor.execute("""
        INSERT INTO runs (date, month, year, start_time, distance, total_time, elevation_gain, avg_speed,
                          avg_steps, total_steps, avg_power, avg_heart_rate, avg_pace, 
                          fastest_pace, slowest_pace, pause, activity, track_img, elevation_img, map_html)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, data + (track_img, elevation_img, map_html))

    conn.commit()
    conn.close()


def insert_activity(date_time, distance, activity_type):
    conn, cursor = connect()

    cursor.execute("""
        INSERT INTO activities (date_time, distance, activity_type)
        VALUES (?, ?, ?)
    """, (date_time, distance, activity_type))
    activity_id = cursor.lastrowid
    conn.commit()
    conn.close()
    return activity_id


def insert_run_details(activity_id, segment_number, heart_rate, speed, pace, pause_time):
    conn, cursor = connect()

    cursor.execute("""
        INSERT INTO run_details (activity_id, segment_number, heart_rate, speed, pace, pause_time)
        VALUES (?, ?, ?, ?, ?, ?)
    """, (activity_id, segment_number, heart_rate, speed, pace, pause_time))
    conn.commit()
    conn.close()


def insert_best_performance(activity_type, distance, best_time, date_time):
    conn, cursor = connect()

    # Check if we already have three records for this distance
    cursor.execute("""
        SELECT id FROM best_performances WHERE activity_type = ? AND distance = ?
        ORDER BY best_time ASC
    """, (activity_type, distance))
    results = cursor.fetchall()

    if len(results) < 3:
        # Insert new best performance
        cursor.execute("""
            INSERT INTO best_performances (activity_type, distance, best_time, date_time)
            VALUES (?, ?, ?, ?)
        """, (activity_type, distance, best_time, date_time))
    else:
        # If the new time is better than the worst recorded time, update the record
        worst_id = results[-1][0]
        cursor.execute("""
            UPDATE best_performances
            SET best_time = ?, date_time = ?
            WHERE id = ?
        """, (best_time, date_time, worst_id))

    conn.commit()
    conn.close()


def update_comment(run_id, comment):
    conn, cursor = connect()

    cursor.execute("UPDATE runs SET comment = ? WHERE id = ?", (comment, run_id))
    conn.commit()
    conn.close()


def update_photo(run_id, photo_path):
    conn, cursor = connect()

    cursor.execute("UPDATE runs SET photo = ? WHERE id = ?", (photo_path, run_id))
    conn.commit()
    conn.close()


def get_years():
    conn, cursor = connect()

    cursor.execute("SELECT DISTINCT year FROM runs ORDER BY year DESC")
    years = [row[0] for row in cursor.fetchall()]

    conn.close()
    return years


def get_months(year):
    conn, cursor = connect()

    cursor.execute("SELECT DISTINCT month FROM runs WHERE year = ? ORDER BY month DESC", (year,))
    months = [row[0] for row in cursor.fetchall()]

    conn.close()
    return months


def get_runs(year, month):
    conn, cursor = connect()

    cursor.execute("SELECT * FROM runs WHERE year = ? AND month = ? ORDER BY date DESC", (year, month))
    runs = cursor.fetchall()

    conn.close()
    return runs


def get_run_by_id(run_id):
    conn, cursor = connect()

    cursor.execute("SELECT * FROM runs WHERE id = ?", (run_id,))
    run = cursor.fetchone()
    conn.close()
    return run
