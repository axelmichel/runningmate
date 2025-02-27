import os
import sqlite3

from processing.system_settings import ViewMode, mapActivityTypes
from utils.logger import logger
from utils.translations import _


class DatabaseHandler:
    """Database handler that allows injecting a connection for better testability and efficiency."""

    SORT_MAP = {
        "date_time": "activities.date",
        "title": "activities.title",
        "duration": "activities.duration",
        "distance": "activities.distance",
        "elevation_gain": "activities.elevation_gain",
    }

    TABLE_COLUMNS = {
        "runs": [
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
        ],
        "cycling": [
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
        ],
        "walking": [
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
        ],
        "activities": [
            "id",
            "distance",
            "activity_type",
            "duration",
            "date",
            "title",
            "file_id",
            "calories",
            "elevation_gain",
        ],
        "activity_details": [
            "activity_id",
            "segment_id",
            "seg_latitude",
            "seg_longitude",
            "seg_avg_heart_rate",
            "seg_avg_power",
            "seg_avg_speed",
            "seg_avg_pace",
            "seg_avg_steps",
            "seg_distance",
            "seg_time_start",
            "seg_time_end",
            "seg_elevation_gain",
        ],
        "weather": [
            "activity_id",
            "max_temp",
            "min_temp",
            "precipitation",
            "max_wind_speed",
            "weather_code",
        ],
    }

    FILTER_MAPPINGS = {
        "min_distance": "activities.distance", "max_distance": "activities.distance",
        "min_duration": "activities.duration", "max_duration": "activities.duration",
        "min_elevation": "activities.elevation_gain", "max_elevation": "activities.elevation_gain",
        "min_date": "activities.date", "max_date": "activities.date",  # Ensure date filtering
    }

    def __init__(self, db_path=None, conn=None):
        """Initialize the database connection."""
        if conn:
            self.conn = conn
        else:
            db_path = db_path or os.path.expanduser("~/RunningData/running_data.db")
            self.conn = sqlite3.connect(db_path, check_same_thread=False)
        self.conn.row_factory = sqlite3.Row  # A
        self.cursor = self.conn.cursor()

    def insert_activity(self, data: dict, segment_df=None):
        self.insert("activities", data)
        if segment_df is not None:
            for segment_id, row in segment_df.iterrows():
                self.insert_activity_details(data["id"], segment_id, row)

    def update_activity(self, data: dict, segment_df=None):
        self.update("activities", data, "id")
        self.cursor.execute(
            "DELETE FROM activity_details WHERE activity_id = ?", (data["id"],)
        )
        if segment_df is not None:
            for segment_id, row in segment_df.iterrows():
                self.insert_activity_details(data["id"], segment_id, row)

    def insert_activity_details(self, activity_id, segment_id, data: dict):
        columns = self.TABLE_COLUMNS["activity_details"]
        identifier = {"activity_id": activity_id, "segment_id": segment_id}
        data = {**data, **identifier}
        self.insert_data("activity_details", columns, data)

    def insert_run(self, data: dict):
        self.insert("runs", data)

    def update_run(self, data: dict):
        self.update("runs", data)

    def insert_cycling(self, data: dict):
        self.insert("cycling", data)

    def update_cycling(self, data: dict):
        self.update("cycling", data)

    def insert_walking(self, data: dict):
        self.insert("walking", data)

    def update_walking(self, data: dict):
        self.update("walking", data)

    def insert_weather(self, data: dict):
        self.insert("weather", data)

    def update_weather(self, data: dict):
        self.update("weather", data)

    def update(self, table, data, id_field="activity_id"):
        columns = self.TABLE_COLUMNS[table]
        self.update_data(table, columns, data, id_field)

    def insert(self, table, data):
        columns = self.TABLE_COLUMNS[table]
        self.insert_data(table, columns, data)

    def insert_data(self, table, columns, data: dict):
        values = [data.get(col, None) for col in columns]

        query = f"""
                  INSERT INTO {table} ({", ".join(columns)})
                  VALUES ({", ".join(["?"] * len(columns))})
              """

        self.cursor.execute(query, values)
        self.conn.commit()

    def update_data(self, table, columns, data: dict, id_field="activity_id"):
        if id_field not in data:
            raise ValueError("activity_id is required for updates.")

        set_clause = ", ".join([f"{col} = ?" for col in columns])
        values = [data.get(col, None) for col in columns]
        values.append(data[id_field])  # Add activity_id for WHERE clause

        query = f"""
            UPDATE {table}
            SET {set_clause}
            WHERE {id_field} = ?
        """

        self.cursor.execute(query, values)
        self.conn.commit()

    def delete_activity(self, activity_id):
        activity = self.fetch_activity(activity_id)
        if not activity:
            return

        target = mapActivityTypes(activity["activity_type"])

        if target == ViewMode.RUN:
            self.cursor.execute(
                "DELETE FROM runs WHERE activity_id = ?", (activity_id,)
            )
        elif target == ViewMode.WALK:
            self.cursor.execute(
                "DELETE FROM walking WHERE activity_id = ?", (activity_id,)
            )
        elif target == ViewMode.CYCLE:
            self.cursor.execute(
                "DELETE FROM cycling WHERE activity_id = ?", (activity_id,)
            )

        self.cursor.execute("DELETE FROM activities WHERE id = ?", (activity_id,))
        self.cursor.execute(
            "DELETE FROM activity_details WHERE activity_id = ?", (activity_id,)
        )
        self.cursor.execute(
            "DELETE FROM weather WHERE weather.activity_id = ?", (activity_id,)
        )
        self.conn.commit()

    def insert_best_performance(
        self, activity_id, activity_type, distance, best_time, date_time
    ):
        """Insert a new best performance record, keeping only the top 3 for each distance."""
        self.cursor.execute(
            """
            SELECT id FROM best_performances WHERE activity_type = ? AND distance = ?
            ORDER BY best_time
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

    def fetch_activity(self, activity_id):
        """
        :param activity_id: The activity ID to fetch
        :return: dictionary
        """
        query = "SELECT * FROM activities WHERE id = ?"
        self.cursor.execute(query, (activity_id,))
        row = self.cursor.fetchone()
        return dict(row)

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
                    activities.file_id,
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
        return highest_id + 1 if highest_id is not None else 1

    def fetch_activities(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC", filters=None
    ):
        """
        Fetch paginated entries from the activities table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :param filters: Dictionary of filters
        :return: List of dictionaries
        """

        if sort_field in self.SORT_MAP:
            sort_field = self.SORT_MAP[sort_field]

        filter_params = None
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
            """
        if filters:
            query += " WHERE 1=1"
            query = self.add_filter_to_query(query, filters)
            filter_params = self.get_filter_params(filters)

        query += f" ORDER BY {sort_field} {sort_direction} LIMIT ? OFFSET ?;"

        return self.fetch_all(query, start, limit, filter_params)

    def fetch_runs(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC", filters=None
    ):
        """
        Fetch paginated entries from the runs table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :param filters: Dictionary of filters
        :return: List of dictionaries
        """

        if sort_field in self.SORT_MAP:
            sort_field = self.SORT_MAP[sort_field]

        filter_params = None

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
               """

        if filters:
            query += " WHERE 1=1"
            query = self.add_filter_to_query(query, filters)
            filter_params = self.get_filter_params(filters)

        query += f" ORDER BY {sort_field} {sort_direction} LIMIT ? OFFSET ?;"

        return self.fetch_all(query, start, limit, filter_params)

    def get_filter_params(self, filters):
        params = []
        search_text = filters.get("search_text", "")
        if search_text:
            params.extend([f"%{search_text}%", f"%{search_text}%"])
        for key,__ in self.FILTER_MAPPINGS.items():
            if key in filters:
                params.append(filters[key])
        return params

    def add_filter_to_query(self, query, filters):
        search_text = filters.get("search_text", "")
        if search_text:
            query += " AND (activities.title LIKE ? OR activities.comment LIKE ?)"
        for key, column in self.FILTER_MAPPINGS.items():
            if key in filters:
                operator = ">=" if "min" in key else "<="
                query += f" AND {column} {operator} ?"
        return query

    def fetch_walks(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC", filters=None
    ):
        """
        Fetch paginated entries from the walks table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :param filters: Dictionary of filters
        :return: List of dictionaries
        """
        filter_params = None

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
           """

        if filters:
            query += " WHERE 1=1"
            query = self.add_filter_to_query(query, filters)
            filter_params = self.get_filter_params(filters)

        query += f" ORDER BY {sort_field} {sort_direction} LIMIT ? OFFSET ?;"

        return self.fetch_all(query, start, limit, filter_params)

    def fetch_rides(
        self, start=0, limit=50, sort_field="date_time", sort_direction="DESC", filters=None
    ):
        """
        Fetch paginated entries from the cycling table, sorting by date (most recent first).

        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50)
        :param sort_field: Column name to sort by (default "activities.date")
        :param sort_direction: Sorting direction (ASC or DESC, default DESC)
        :param filters: Dictionary of filters
        :return: List of dictionaries
        """
        filter_params = None

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
              """

        if filters:
            query += " WHERE 1=1"
            query = self.add_filter_to_query(query, filters)
            filter_params = self.get_filter_params(filters)

        query += f" ORDER BY {sort_field} {sort_direction} LIMIT ? OFFSET ?;"

        return self.fetch_all(query, start, limit, filter_params)

    def fetch_all(self, query, start=0, limit=50, filter_params=None):
        """
        Fetch paginated entries from the database using a custom query.
        :param start: Offset for pagination (default: 0)
        :param limit: Number of records to fetch (default: 50
        :param filter_params: List of additional parameters for the query
        :return: List of dictionaries
        """
        if filter_params is None:
            filter_params = []

        params = filter_params + [limit, start]

        print (f" params: {params}")
        print (f" query: {query}")

        self.cursor.execute(query, params)
        rows = self.cursor.fetchall()

        return [dict(row) for row in rows]

    def get_total_activity_count(self, view_mode: ViewMode):
        """Get the total number of activities for a given view mode."""
        if view_mode == ViewMode.RUN:
            table = "runs"
        elif view_mode == ViewMode.WALK:
            table = "walking"
        elif view_mode == ViewMode.CYCLE:
            table = "cycling"
        else:
            table = "activities"

        self.cursor.execute(f"SELECT COUNT(*) FROM {table}")
        return self.cursor.fetchone()[0]

    def get_activity_by_file_id(self, file_id):
        """
        Fetch an activity ID by file ID.
        :param file_id: The file ID to fetch
        :return: int or None
        """
        self.cursor.execute("SELECT id FROM activities WHERE file_id = ?", (file_id,))
        result = self.cursor.fetchone()
        return result[0] if result else None

    def delete_media(self, activity_id, file_path):
        """Deletes a media entry from the database and removes the file."""
        try:
            query = "DELETE FROM media WHERE activity_id = ? AND file_path = ?"
            self.cursor.execute(query, (activity_id, file_path))
            self.conn.commit()

            # Remove the actual file if it exists
            if os.path.exists(file_path):
                os.remove(file_path)
                logger.debug(f"Deleted file: {file_path}")
            else:
                logger.warning(f"File not found: {file_path}")

        except Exception as e:
            logger.error(f"Error deleting media: {e}")

    def close(self):
        """Close the database connection."""
        self.conn.close()
