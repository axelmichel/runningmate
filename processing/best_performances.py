from typing import Any, Dict, List, Optional

import pandas as pd

from database.database_handler import DatabaseHandler
from processing.system_settings import ViewMode


class BestSegmentFinder:
    """
    A class to find the best (fastest) segment for predefined distances in running, walking, and cycling activities.
    It also caches these segments in a best_performances table for later fast retrieval.

    Attributes:
        conn (sqlite3.Connection): A SQLite database connection.
    """

    # Define segment distances per activity type
    SEGMENT_DISTANCES = {
        ViewMode.RUN: [1, 5, 10, 21, 42],  # Distances in KM for running
        ViewMode.WALK: [1, 5, 10, 15, 20, 30, 50],  # Walking
        ViewMode.CYCLE: [5, 25, 50, 75, 100, 150, 200],  # Cycling
    }

    def __init__(self, db_handler: DatabaseHandler):
        """
        Initializes the BestSegmentFinder class.

        :param db_handler: DatabaseHandler
        """
        self.conn = db_handler.conn

    def get_best_segments(
        self, activity_id: int, activity_type: ViewMode
    ) -> Optional[Dict[str, Any]]:
        """
        Retrieves the best (fastest) segment per predefined distance for a given activity.
        First, it checks if data exists in the best_performances table.
        If not, it computes the best segments from activity_details, stores them in best_performances, and returns the result.
        """
        if activity_type not in self.SEGMENT_DISTANCES:
            return None  # Unsupported activity type

        # Try retrieving from best_performances
        query_cached = """
            SELECT distance, best_time, date_time
            FROM best_performances
            WHERE activity_id = ? AND activity_type = ?;
        """
        df_cached = pd.read_sql(
            query_cached, self.conn, params=(activity_id, activity_type)
        )
        if not df_cached.empty:
            best_segments = {}
            for _, row in df_cached.iterrows():
                # Note: best_performances does not store seg_time_end.
                best_segments[f"{int(row['distance'])}K"] = {
                    "seg_time_start": row["date_time"],
                    "seg_avg_pace": row["best_time"],
                }
            return best_segments

        # Otherwise, run the original query on activity_details
        query_details = """
            SELECT segment_id, seg_distance, seg_time_start, seg_time_end, seg_avg_pace
            FROM activity_details
            WHERE activity_id = ?
            ORDER BY segment_id ASC;
        """
        df_details = pd.read_sql(query_details, self.conn, params=(activity_id,))
        if df_details.empty:
            return None  # No segments found

        best_segments = {}
        for target_distance in self.SEGMENT_DISTANCES[activity_type]:
            target_distance = int(target_distance)
            best_segment = self._get_best_cumulative_segment(
                df_details, target_distance
            )
            if best_segment:
                best_segments[f"{int(target_distance)}K"] = best_segment
                # Insert the computed best segment into the best_performances table.
                # Fields: activity_id, distance, best_time (pace), date_time (seg_time_start), activity_type.
                insert_query = """
                    INSERT INTO best_performances (activity_id, distance, best_time, date_time, activity_type)
                    VALUES (?, ?, ?, ?, ?);
                """
                self.conn.execute(
                    insert_query,
                    (
                        activity_id,
                        target_distance,
                        best_segment["seg_avg_pace"],
                        best_segment["seg_time_start"],
                        activity_type,
                    ),
                )
                self.conn.commit()
        return best_segments if best_segments else None

    @staticmethod
    def _get_best_cumulative_segment(
        df: pd.DataFrame, target_distance: int
    ) -> Optional[Dict[str, Any]]:
        """
        Finds the fastest segment by summing smaller segments until reaching the target distance.

        Instead of recalculating pace, this method simply averages the existing segment paces.
        """

        best_segment = None
        best_pace = float("inf")

        for start_idx in range(len(df)):
            total_distance = 0.0
            pace_sum = 0.0
            segment_count = 0
            segment_start_time = df.iloc[start_idx]["seg_time_start"]

            for end_idx in range(start_idx, len(df)):
                segment = df.iloc[end_idx]
                segment_distance = (
                    segment["seg_distance"]
                    if segment["seg_distance"] is not None
                    else 0.0
                )

                total_distance += segment_distance
                pace_sum += segment["seg_avg_pace"]
                segment_count += 1

                if total_distance < target_distance:
                    continue

                avg_pace = pace_sum / segment_count

                if avg_pace < best_pace:
                    best_pace = avg_pace
                    best_segment = {
                        "seg_time_start": segment_start_time,
                        "seg_time_end": segment["seg_time_end"],
                        "seg_avg_pace": avg_pace,
                    }

                break  # Stop searching this start index once we hit the target distance

        return best_segment

    def get_best_performance(
        self, activity_type: ViewMode
    ) -> Optional[Dict[str, List[Dict[str, Any]]]]:
        """
        Retrieves the overall best performances across all activities for a given activity type.
        Groups them by segment distance.
        """
        if activity_type not in self.SEGMENT_DISTANCES:
            return None  # Unsupported activity type

        best_performances = {}

        for target_distance in self.SEGMENT_DISTANCES[activity_type]:
            query = """
                SELECT activity_id, distance, best_time, date_time
                FROM best_performances
                WHERE activity_type = ? AND distance = ?
                ORDER BY best_time ASC
                LIMIT 3;
            """
            df_perf = pd.read_sql(
                query, self.conn, params=(activity_type, target_distance)
            )

            if not df_perf.empty:
                best_performances[f"{int(target_distance)}K"] = [
                    {
                        "activity_id": row["activity_id"],
                        "seg_time_start": row["date_time"],
                        "seg_avg_pace": row["best_time"],
                    }
                    for _, row in df_perf.iterrows()
                ]

        return best_performances if best_performances else None
