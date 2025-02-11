import sqlite3
import os


class DatabaseHandler:
    """Database handler that allows injecting a connection for better testability and efficiency."""

    def __init__(self, db_path=None, conn=None):
        """Initialize the database connection."""
        if conn:
            self.conn = conn  # ✅ Use injected connection (for tests)
        else:
            db_path = db_path or os.path.expanduser("~/RunningData/running_data.db")
            self.conn = sqlite3.connect(db_path, check_same_thread=False)
        self.cursor = self.conn.cursor()

    def insert_run(self, data, track_img, elevation_img, map_html):
        """Insert a new run record."""
        self.cursor.execute("""
            INSERT INTO runs (date, month, year, start_time, distance, total_time, elevation_gain, avg_speed,
                              avg_steps, total_steps, avg_power, avg_heart_rate, avg_pace, 
                              fastest_pace, slowest_pace, pause, activity, track_img, elevation_img, map_html)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, data + (track_img, elevation_img, map_html))
        self.conn.commit()

    def insert_activity(self, date_time, distance, activity_type):
        """Insert a new activity record and return its ID."""
        self.cursor.execute("""
            INSERT INTO activities (date_time, distance, activity_type)
            VALUES (?, ?, ?)
        """, (date_time, distance, activity_type))
        self.conn.commit()
        return self.cursor.lastrowid

    def insert_run_details(self, activity_id, segment_number, heart_rate, speed, pace, pause_time):
        """Insert run segment details."""
        self.cursor.execute("""
            INSERT INTO run_details (activity_id, segment_number, heart_rate, speed, pace, pause_time)
            VALUES (?, ?, ?, ?, ?, ?)
        """, (activity_id, segment_number, heart_rate, speed, pace, pause_time))
        self.conn.commit()

    def insert_best_performance(self, activity_id, activity_type, distance, best_time, date_time):
        """Insert a new best performance record, keeping only the top 3 for each distance."""
        self.cursor.execute("""
            SELECT id FROM best_performances WHERE activity_type = ? AND distance = ?
            ORDER BY best_time ASC
        """, (activity_type, distance))
        results = self.cursor.fetchall()

        if len(results) < 3:
            # Insert new best performance
            self.cursor.execute("""
                INSERT INTO best_performances (activity_id, activity_type, distance, best_time, date_time)
                VALUES (?, ?, ?, ?, ?)
            """, (activity_id, activity_type, distance, best_time, date_time))
        else:
            # If the new time is better than the worst recorded time, update the record
            worst_id = results[-1][0]
            self.cursor.execute("""
                UPDATE best_performances
                SET best_time = ?, date_time = ?
                WHERE id = ?
            """, (best_time, date_time, worst_id))

        self.conn.commit()

    def insert_media(self, activity_id, media_type, file_path):
        self.cursor.execute("""
            INSERT INTO media (activity_id, media_type, file_path) 
            VALUES (?, ?, ?)
        """, (activity_id, media_type, file_path))
        self.conn.commit()

    def update_comment(self, activity_id, comment):
        self.cursor.execute("UPDATE activities SET comment = ? WHERE id = ?", (comment, activity_id))
        self.conn.commit()

    def update_photo(self, run_id, photo_path):
        """Update the photo path for a specific run."""
        self.cursor.execute("UPDATE runs SET photo = ? WHERE id = ?", (photo_path, run_id))
        self.conn.commit()

    def get_years(self):
        """Get distinct years from runs."""
        self.cursor.execute("SELECT DISTINCT year FROM runs ORDER BY year DESC")
        return [row[0] for row in self.cursor.fetchall()]

    def get_months(self, year):
        """Get distinct months for a given year."""
        self.cursor.execute("SELECT DISTINCT month FROM runs WHERE year = ? ORDER BY month DESC", (year,))
        return [row[0] for row in self.cursor.fetchall()]

    def get_runs(self, year, month):
        """Get all runs for a given year and month."""
        self.cursor.execute("SELECT * FROM runs WHERE year = ? AND month = ? ORDER BY date DESC", (year, month))
        return self.cursor.fetchall()

    def get_run_by_id(self, run_id):
        """Retrieve a specific run by ID."""
        self.cursor.execute("SELECT * FROM runs WHERE id = ?", (run_id,))
        return self.cursor.fetchone()

    def get_comment(self, activity_id):
        self.cursor.execute("SELECT comment FROM activities WHERE id = ?", (activity_id,))
        result = self.cursor.fetchone()
        return result[0] if result else ""

    def get_media_files(self, activity_id):
        self.cursor.execute("""
               SELECT id, media_type, file_path FROM media
               WHERE activity_id = ?
           """, (activity_id,))
        return self.cursor.fetchall()

    def delete_media(self, activity_id, file_path):
        """Deletes a media entry from the database and removes the file."""
        try:
            query = "DELETE FROM media WHERE activity_id = ? AND file_path = ?"
            self.cursor.execute(query, (activity_id, file_path))
            self.conn.commit()

            # Remove the actual file if it exists
            if os.path.exists(file_path):
                os.remove(file_path)
                print(f"Deleted file: {file_path}")
            else:
                print(f"File not found: {file_path}")

        except Exception as e:
            print(f"Error deleting media: {e}")

    def close(self):
        """Close the database connection."""
        self.conn.close()
