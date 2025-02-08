import sqlite3
import os

DB_PATH = os.path.expanduser("~/RunningData/running_data.db")


def initialize_database():
    """Initialize the database with the required schema."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("""
            CREATE TABLE IF NOT EXISTS runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                month TEXT,
                year TEXT,
                start_time TEXT NOT NULL,
                distance REAL NOT NULL,
                total_time TEXT NOT NULL,
                elevation_gain INTEGER NOT NULL,
                avg_speed REAL NOT NULL,
                avg_steps INTEGER,
                total_steps INTEGER,
                avg_power INTEGER,
                avg_heart_rate INTEGER,
                avg_pace TEXT,
                fastest_pace TEXT,
                slowest_pace TEXT,
                pause TEXT,
                activity TEXT NOT NULL,
                track_img TEXT,
                elevation_img TEXT,
                map_html TEXT,
                comment TEXT,
                photo TEXT
                
            );
        """)

    conn.commit()
    conn.close()


def insert_run(data, track_img, elevation_img, map_html):
    """Inserts computed run statistics into the SQLite database."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("""
        INSERT INTO runs (date, month, year, start_time, distance, total_time, elevation_gain, avg_speed,
                          avg_steps, total_steps, avg_power, avg_heart_rate, avg_pace, 
                          fastest_pace, slowest_pace, pause, activity, track_img, elevation_img, map_html)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, data + (track_img, elevation_img, map_html))

    conn.commit()
    conn.close()

def update_comment(run_id, comment):
    """Update a run's comment in the database."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("UPDATE runs SET comment = ? WHERE id = ?", (comment, run_id))
    conn.commit()
    conn.close()

def update_photo(run_id, photo_path):
    """Update a run's photo in the database."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("UPDATE runs SET photo = ? WHERE id = ?", (photo_path, run_id))
    conn.commit()
    conn.close()


def get_years():
    """Returns available years from the database."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("SELECT DISTINCT year FROM runs ORDER BY year DESC")
    years = [row[0] for row in cursor.fetchall()]

    conn.close()
    return years


def get_months(year):
    """Returns available months for a given year."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("SELECT DISTINCT month FROM runs WHERE year = ? ORDER BY month DESC", (year,))
    months = [row[0] for row in cursor.fetchall()]

    conn.close()
    return months


def get_runs(year, month):
    """Returns all runs for a given month and year."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    cursor.execute("SELECT * FROM runs WHERE year = ? AND month = ? ORDER BY date DESC", (year, month))
    runs = cursor.fetchall()

    conn.close()
    return runs

def get_run_by_id(run_id):
    """Fetch a single run's details by its ID."""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM runs WHERE id = ?", (run_id,))
    run = cursor.fetchone()
    conn.close()
    return run