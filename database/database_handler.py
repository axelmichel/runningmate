import os
import sqlite3

from utils.translations import _


class DatabaseHandler:
    """Database handler that allows injecting a connection for better testability and efficiency."""

    SORT_MAP = {
        "date_time": "activities.date",
        "title": "activities.title",
        "duration": "activities.duration",
        "distance": "activities.distance",
    }

    def __init__(self, db_path=None, conn=None):
        """Initialize the database connection."""
        if conn:
            self.conn = conn  # ✅ Use injected connection (for tests)
        else:
            db_path = db_path or os.path.expanduser("~/RunningData/running_data.db")
            self.conn = sqlite3.connect(db_path, check_same_thread=False)
        self.conn.row_factory = sqlite3.Row  # A
        self.cursor = self.conn.cursor()

    def insert_run(self, data: dict):
        columns = [
            "activity_id",
            "elevation_gain",
            "avg_speed",
            "avg_steps",
            "total_steps",
            "avg_power",
            "avg_heart_rate",
            "avg_pace",
            "fastest_pace",
            "slowest_pace",
            "pause",
            "track_img",
            "elevation_img",
            "map_html",
        ]
        self.insert_data("runs", columns, data)

    def insert_cycling(self, data: dict):
        columns = [
            "activity_id",
            "elevation_gain",
            "avg_speed",
            "avg_power",
            "avg_heart_rate",
            "avg_pace",
            "fastest_pace",
            "slowest_pace",
            "pause",
            "track_img",
            "elevation_img",
            "map_html",
        ]
        self.insert_data("cycling", columns, data)

    def insert_walk(self, data: dict):
        columns = [
            "activity_id",
            "elevation_gain",
            "avg_speed",
            "avg_steps",
            "total_steps",
            "avg_power",
            "avg_heart_rate",
            "avg_pace",
            "fastest_pace",
            "slowest_pace",
            "pause",
            "track_img",
            "elevation_img",
            "map_html",
        ]
        self.insert_data("walking", columns, data)

    def insert_activity(self, data: dict):
        columns = ["id", "distance", "activity_type", "duration", "date", "title"]
        self.insert_data("activities", columns, data)

    def insert_data(self, table, columns, data: dict):
        values = [data.get(col, None) for col in columns]

        query = f"""
                  INSERT INTO {table} ({", ".join(columns)})
                  VALUES ({", ".join(["?"] * len(columns))})
              """

        self.cursor.execute(query, values)
        self.conn.commit()

    def insert_run_details(
        self, activity_id, segment_number, heart_rate, speed, pace, pause_time
    ):
        """Insert run segment details."""
        self.cursor.execute(
            """
            INSERT INTO run_details (activity_id, segment_number, heart_rate, speed, pace, pause_time)
            VALUES (?, ?, ?, ?, ?, ?)
        """,
            (activity_id, segment_number, heart_rate, speed, pace, pause_time),
        )
        self.conn.commit()

    def insert_best_performance(
        self, activity_id, activity_type, distance, best_time, date_time
    ):
        """Insert a new best performance record, keeping only the top 3 for each distance."""
        self.cursor.execute(
            """
            SELECT id FROM best_performances WHERE activity_type = ? AND distance = ?
            ORDER BY best_time ASC
        """,
            (activity_type, distance),
        )
        results = self.cursor.fetchall()

        if len(results) < 3:
            # Insert new best performance
            self.cursor.execute(
                """
                INSERT INTO best_performances (activity_id, activity_type, distance, best_time, date_time)
                VALUES (?, ?, ?, ?, ?)
            """,
                (activity_id, activity_type, distance, best_time, date_time),
            )
        else:
            # If the new time is better than the worst recorded time, update the record
            worst_id = results[-1][0]
            self.cursor.execute(
                """
                UPDATE best_performances
                SET best_time = ?, date_time = ?
                WHERE id = ?
            """,
                (best_time, date_time, worst_id),
            )

        self.conn.commit()

    def insert_media(self, activity_id, media_type, file_path):
        self.cursor.execute(
            """
            INSERT INTO media (activity_id, media_type, file_path) 
            VALUES (?, ?, ?)
        """,
            (activity_id, media_type, file_path),
        )
        self.conn.commit()

    def update_comment(self, activity_id, comment):
        self.cursor.execute(
            "UPDATE activities SET comment = ? WHERE id = ?", (comment, activity_id)
        )
        self.conn.commit()

    def get_comment(self, activity_id):
        self.cursor.execute(
            "SELECT comment FROM activities WHERE id = ?", (activity_id,)
        )
        result = self.cursor.fetchone()
        return result[0] if result else ""

    def get_media_files(self, activity_id):
        self.cursor.execute(
            """
               SELECT id, media_type, file_path FROM media
               WHERE activity_id = ?
           """,
            (activity_id,),
        )
        return self.cursor.fetchall()

    def fetch_run_by_activity_id(self, activity_id):
        """
        Fetch a run, joining the activities table to get some basic information.
        :param activity_id: The activity ID to fetch
        :return: dictionary
        """
        query = f"""
                SELECT 
                    runs.id,
                    strftime('{_("%d.%m.%Y")}', activities.date, 'unixepoch') AS date,
                    strftime('%H:%M', activities.date, 'unixepoch') AS time,
                    printf('%02d:%02d:%02d', activities.duration / 3600, (activities.duration % 3600) / 60, activities.duration % 60) AS duration,
                    activities.distance,
                    activities.title,
                    activities.comment,
                    activities.activity_type,
                    runs.activity_id,
                    runs.elevation_gain,
                    runs.avg_speed,
                    runs.avg_steps,
                    runs.total_steps,
                    runs.avg_power,
                    runs.avg_heart_rate,
                    runs.avg_pace,
                    runs.fastest_pace,
                    runs.slowest_pace,
                    runs.pause,
                    runs.track_img,
                    runs.elevation_img,
                    runs.map_html,
                    shoes.name as shoe_name,
                    shoes.distance as shoe_distance,
                    shoes.status as shoe_status
                FROM runs
                JOIN activities ON activities.id = runs.activity_id
                LEFT JOIN shoes ON shoes.id = runs.shoe_id AND runs.shoe_id IS NOT NULL 
                WHERE runs.activity_id = ?;
            """

        self.cursor.execute(query, (activity_id,))
        row = self.cursor.fetchone()
        return dict(row)

    def fetch_walk_by_activity_id(self, activity_id):
        """
        Fetch a run, joining the activities table to get some basic information.
        :param activity_id: The activity ID to fetch
        :return: dictionary
        """
        query = f"""
                SELECT 
                    walking.id,
                    strftime('{_("%d.%m.%Y")}', activities.date, 'unixepoch') AS date,
                    strftime('%H:%M', activities.date, 'unixepoch') AS time,
                    printf('%02d:%02d:%02d', activities.duration / 3600, (activities.duration % 3600) / 60, activities.duration % 60) AS duration,
                    activities.distance,
                    activities.title,
                    activities.comment,
                    activities.activity_type,
                    walking.activity_id,
                    walking.elevation_gain,
                    walking.avg_speed,
                    walking.avg_steps,
                    walking.total_steps,
                    walking.avg_power,
                    walking.avg_heart_rate,
                    walking.avg_pace,
                    walking.fastest_pace,
                    walking.slowest_pace,
                    walking.pause,
                    walking.track_img,
                    walking.elevation_img,
                    walking.map_html
                FROM walking
                JOIN activities ON activities.id = walking.activity_id
                WHERE walking.activity_id = ?;
            """

        self.cursor.execute(query, (activity_id,))
        row = self.cursor.fetchone()
        return dict(row)

    def fetch_ride_by_activity_id(self, activity_id):
        """
        Fetch a run, joining the activities table to get some basic information.
        :param activity_id: The activity ID to fetch
        :return: dictionary
        """
        query = f"""
                   SELECT 
                       cycling.id,
                       strftime('{_("%d.%m.%Y")}', activities.date, 'unixepoch') AS date,
                       strftime('%H:%M', activities.date, 'unixepoch') AS time,
                       printf('%02d:%02d:%02d', activities.duration / 3600, (activities.duration % 3600) / 60, activities.duration % 60) AS duration,
                       activities.distance,
                       activities.title,
                       activities.comment,
                       activities.activity_type,
                       cycling.activity_id,
                       cycling.elevation_gain,
                       cycling.avg_speed,
                       cycling.avg_power,
                       cycling.avg_heart_rate,
                       cycling.avg_pace,
                       cycling.fastest_pace,
                       cycling.slowest_pace,
                       cycling.pause,
                       cycling.track_img,
                       cycling.elevation_img,
                       cycling.map_html
                   FROM cycling
                   JOIN activities ON activities.id = cycling.activity_id
                   WHERE cycling.activity_id = ?;
               """

        self.cursor.execute(query, (activity_id,))
        row = self.cursor.fetchone()
        return dict(row)

    def get_next_activity_id(self):
        query = "SELECT MAX(id) FROM activities;"
        self.cursor.execute(query)
        highest_id = self.cursor.fetchone()[0]
        return highest_id + 1 if highest_id is not None else 1  #

    def fetch_activities(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC"
    ):
        """
        Fetch paginated entries from the activities table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :return: List of dictionaries
        """

        if sort_field in self.SORT_MAP:
            sort_field = self.SORT_MAP[sort_field]

        query = f"""
            SELECT 
                id as activity_id, 
                strftime('{_("%d.%m.%Y")}', date, 'unixepoch') AS date_time,  -- YYYY.MM.DD format
                strftime('%H:%M', date, 'unixepoch') AS time,  -- HH:MM format
                printf('%02d:%02d:%02d', duration / 3600, (duration % 3600) / 60, duration % 60) AS duration,
                activity_type, 
                duration,
                distance,
                title 
            FROM activities 
            ORDER BY {sort_field} {sort_direction}
            LIMIT ? OFFSET ?;
            """

        return self.fetch_all(query, start, limit)

    def fetch_runs(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC"
    ):
        """
        Fetch paginated entries from the runs table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :return: List of dictionaries
        """

        if sort_field in self.SORT_MAP:
            sort_field = self.SORT_MAP[sort_field]

        query = f"""
               SELECT 
                    runs.id,
                    runs.activity_id,
                    strftime('{_("%d.%m.%Y %H:%M")}', activities.date, 'unixepoch') AS date_time,
                    printf('%02d:%02d:%02d', activities.duration / 3600, (activities.duration % 3600) / 60, activities.duration % 60) AS duration,
                    activities.distance,
                    activities.title,
                    activities.activity_type,
                    runs.elevation_gain,
                    runs.avg_speed,
                    runs.avg_steps,
                    runs.total_steps,
                    runs.avg_power,
                    runs.avg_heart_rate,
                    runs.avg_pace,
                    runs.fastest_pace,
                    runs.slowest_pace,
                    runs.pause
                FROM runs
                JOIN activities ON activities.id = runs.activity_id
                ORDER BY {sort_field} {sort_direction}
                LIMIT ? OFFSET ?;
               """

        return self.fetch_all(query, start, limit)

    def fetch_walks(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC"
    ):
        """
        Fetch paginated entries from the walks table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :return: List of dictionaries
        """

        if sort_field in self.SORT_MAP:
            sort_field = self.SORT_MAP[sort_field]

        query = f"""
           SELECT 
                walking.id,
                walking.activity_id,
                strftime('{_("%d.%m.%Y %H:%M")}', activities.date, 'unixepoch') AS date_time,
                printf('%02d:%02d:%02d', activities.duration / 3600, (activities.duration % 3600) / 60, activities.duration % 60) AS duration,
                activities.distance,
                activities.title,
                activities.activity_type,
                walking.elevation_gain,
                walking.avg_speed,
                walking.avg_steps,
                walking.total_steps,
                walking.avg_power,
                walking.avg_heart_rate,
                walking.avg_pace,
                walking.fastest_pace,
                walking.slowest_pace,
                walking.pause
            FROM walking
            JOIN activities ON activities.id = walking.activity_id
            ORDER BY {sort_field} {sort_direction}
            LIMIT ? OFFSET ?;
           """

        return self.fetch_all(query, start, limit)

    def fetch_rides(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC"
    ):
        """
        Fetch paginated entries from the cycling table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :return: List of dictionaries
        """

        if sort_field in self.SORT_MAP:
            sort_field = self.SORT_MAP[sort_field]

        query = f"""
              SELECT 
                   cycling.id,
                   cycling.activity_id,
                   strftime('{_("%d.%m.%Y %H:%M")}', activities.date, 'unixepoch') AS date_time,
                   printf('%02d:%02d:%02d', activities.duration / 3600, (activities.duration % 3600) / 60, activities.duration % 60) AS duration,
                   activities.distance,
                   activities.title,
                   activities.activity_type,
                   cycling.elevation_gain,
                   cycling.avg_speed,
                   cycling.avg_power,
                   cycling.avg_heart_rate,
                   cycling.avg_pace,
                   cycling.fastest_pace,
                   cycling.slowest_pace,
                   cycling.pause
               FROM cycling
               JOIN activities ON activities.id = cycling.activity_id
               ORDER BY {sort_field} {sort_direction}
               LIMIT ? OFFSET ?;
              """

        return self.fetch_all(query, start, limit)

    def fetch_all(self, query, start=0, limit=50):
        """
        Fetch paginated entries from the database using a custom query.
        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50
        :return: List of dictionaries
        """
        self.cursor.execute(query, (limit, start))
        rows = self.cursor.fetchall()

        return [dict(row) for row in rows]

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
