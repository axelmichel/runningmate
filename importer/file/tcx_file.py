import os
import tarfile

import numpy as np
import pandas as pd
from PyQt6.QtWidgets import QFileDialog

from database.database_handler import DatabaseHandler
from processing.compute_statistics import format_hour_minute, generate_activity_title
from processing.data_processing import (
    calculate_distance,
    calculate_pace,
    calculate_steps,
    convert_to_utm,
    detect_pauses,
)
from processing.parse_tcx import parse_tcx
from processing.system_settings import ViewMode, mapActivityTypes
from processing.visualization import plot_activity_map, plot_elevation, plot_track
from utils import logger


class TcxFileImporter:
    def __init__(self, file_path, image_path, db_handler: DatabaseHandler):
        super().__init__()
        self.file_path = file_path
        self.image_path = image_path
        self.db = db_handler

    def upload(self):
        file_path, _ = QFileDialog.getOpenFileName(
            None, "Select TCX File", "", "TCX Files (*.tcx)"
        )

        if file_path:
            self.process_file(file_path)
            self.archive_file(file_path)
            return True
        return False

    def process_file(self, file_path):
        df, activity_type = parse_tcx(file_path)
        df = convert_to_utm(df)
        df = calculate_distance(df)
        df, avg_pace, fastest_pace, slowest_pace = calculate_pace(df, ViewMode.RUN)

        name = os.path.basename(file_path).replace(".tcx", "")

        target = mapActivityTypes(activity_type)
        next_id = self.db.get_next_activity_id()

        computed_data = self.compute_data(df)
        computed_data["title"] = generate_activity_title(
            mapActivityTypes(activity_type), computed_data["date"]
        )
        computed_data["activity_id"] = next_id
        computed_data["activity_type"] = activity_type
        computed_data["avg_pace"] = format_hour_minute(avg_pace)
        computed_data["fastest_pace"] = format_hour_minute(fastest_pace)
        computed_data["slowest_pace"] = format_hour_minute(slowest_pace)
        computed_data["pause"] = format_hour_minute(detect_pauses(df))

        if target == ViewMode.RUN:
            computed_data = self.plot_stats(name, df, computed_data)
            self.process_run(df, computed_data)
        elif target == ViewMode.WALK:
            computed_data = self.plot_stats(name, df, computed_data)
            self.process_walk(df, computed_data)
        elif target == ViewMode.CYCLE:
            computed_data = self.plot_stats(name, df, computed_data)
            self.process_cycle(df, computed_data)
        else:
            return False

        computed_data["id"] = computed_data["activity_id"]
        self.db.insert_activity(computed_data)

    def process_run(self, df, computed_data):
        avg_steps, total_steps = calculate_steps(df)
        computed_data["avg_steps"] = (
            int(round(avg_steps, 0)) if not np.isnan(avg_steps) else 0
        )
        computed_data["total_steps"] = (
            int(total_steps) if not np.isnan(total_steps) else 0
        )

        print(f"Computed Data Before Insert: {computed_data}")

        self.db.insert_run(computed_data)

    def process_walk(self, df, computed_data):
        avg_steps, total_steps = calculate_steps(df)
        computed_data["avg_steps"] = (
            int(round(avg_steps, 0)) if not np.isnan(avg_steps) else 0
        )
        computed_data["total_steps"] = (
            int(total_steps) if not np.isnan(total_steps) else 0
        )
        self.db.insert_walk(computed_data)

    def process_cycle(self, df, computed_data):
        self.db.insert_cycling(computed_data)

    def plot_stats(self, name, df, computed_data):
        track_img = os.path.join(self.image_path, f"{name}_track.png")
        elevation_img = os.path.join(self.image_path, f"{name}_elevation.svg")
        map_html = os.path.join(self.image_path, f"{name}_map.html")

        try:
            plot_track(df, track_img)
            computed_data["track_img"] = track_img
        except Exception as e:
            logger.warning(f"Failed to generate track image: {e}")

        try:
            plot_elevation(df, elevation_img)
            computed_data["elevation_img"] = elevation_img
        except Exception as e:
            logger.warning(f"Failed to generate elevation image: {e}")

        try:
            plot_activity_map(df, map_html)
            computed_data["map_html"] = map_html
        except Exception as e:
            logger.warning(f"Failed to generate activity map: {e}")

        return computed_data

    def archive_file(self, file_path):
        base_name = os.path.basename(file_path)  # Extract file name
        tar_gz_filename = os.path.join(
            self.file_path, f"{base_name}.tar.gz"
        )  # Output tar.gz file

        with tarfile.open(tar_gz_filename, "w:gz") as tar:
            tar.add(
                file_path, arcname=base_name
            )  # Store file inside archive without full path

    @staticmethod
    def compute_data(df):
        total_distance = df["Distance"].iloc[-1] if "Distance" in df.columns else 0
        total_time = (
            (
                pd.to_datetime(df["Time"].iloc[-1]) - pd.to_datetime(df["Time"].iloc[0])
            ).total_seconds()
            if "Time" in df.columns
            else 0
        )
        avg_speed = total_distance / (total_time / 3600) if total_time > 0 else 0

        return {
            "date": pd.to_datetime(df["Time"].iloc[0], utc=True).timestamp(),
            "distance": round(total_distance, 2),
            "duration": total_time,
            "elevation_gain": (
                int(round(df["Elevation"].diff().clip(lower=0).sum(), 0))
                if "Elevation" in df.columns
                else 0
            ),
            "avg_heart_rate": (
                int(round(df["HeartRate"].mean(), 0))
                if not df["HeartRate"].isnull().all()
                else 0
            ),
            "avg_speed": round(avg_speed, 2),
            "avg_power": (
                int(round(df["Power"].mean(), 0))
                if not df["Power"].isnull().all()
                else 0
            ),
        }
