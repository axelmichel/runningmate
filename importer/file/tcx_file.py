import os
import tarfile

import numpy as np
import pandas as pd
from PyQt6.QtCore import QThread, pyqtSignal
from PyQt6.QtWidgets import QFileDialog, QMessageBox

from database.database_handler import DatabaseHandler
from processing.compute_statistics import format_hour_minute, generate_activity_title
from processing.data_processing import (
    calculate_distance,
    calculate_pace,
    calculate_steps,
    convert_to_utm,
    detect_pauses,
)
from processing.system_settings import ViewMode, mapActivityTypes
from processing.tcx_file_parser import TcxFileParser
from processing.tcx_segment_parser import TcxSegmentParser
from processing.visualization import plot_activity_map, plot_elevation, plot_track
from processing.weather import WeatherService
from utils.logger import logger
from utils.save_round import safe_round


def get_weather_segment(details):
    if details.empty:
        return None  # Handle empty list case

    middle_index = len(details) // 2
    middle_segment = details.iloc[middle_index]

    return {
        "latitude": middle_segment["seg_latitude"],
        "longitude": middle_segment["seg_longitude"],
        "time": middle_segment["seg_time_start"].split(" ")[0],
    }


class TcxFileImporter:
    def __init__(self, file_path, image_path, db_handler: DatabaseHandler, parent=None):
        super().__init__()
        self.file_path = file_path
        self.image_path = image_path
        self.db = db_handler
        self.parent = parent

    def by_upload(self, activity_id=None):
        file_path, _ = QFileDialog.getOpenFileName(
            None, "Select TCX File", "", "TCX Files (*.tcx)"
        )

        if file_path:
            self.process_file(file_path, activity_id)
            self.archive_file(file_path)
            return True
        return False

    def by_file(self, file_path, activity_id=None):
        if file_path:
            self.process_file(file_path, activity_id)
            self.archive_file(file_path)
            os.remove(file_path)
            return True
        return

    def by_activity(self, activity_id):
        proceed = False
        tcx_path = None
        activity = self.db.fetch_activity(activity_id)
        logger.debug(f"activity: {activity}")
        if activity:
            file_id = activity.get("file_id")
            if file_id:
                file_path = os.path.join(self.file_path, f"{file_id}.tcx.tar.gz")
                tcx_path = os.path.join(self.file_path, f"{file_id}.tcx")
                if os.path.exists(file_path):
                    proceed = self.unpack_tar(file_path)
        if proceed:
            logger.debug(f"activity found, using file: {tcx_path}")
            self.by_file(tcx_path, activity_id)
            return True
        if not proceed:
            return self.prompt_for_upload(activity_id)

    def unpack_tar(self, file_path):
        extract_dir = os.path.dirname(file_path)  # Extract in the same directory
        try:
            with tarfile.open(file_path, "r:gz") as tar:
                tar.extractall(path=extract_dir)
                return True

        except Exception as e:
            logger.critical(f"Failed to extract tar.gz: {e}")
            return False

    def prompt_for_upload(self, activity_id):
        """Prompts the user for file upload if the tar.gz is missing."""
        reply = QMessageBox.question(
            self.parent,
            "File Not Found",
            "The expected file is missing. Do you want to upload a new file?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )

        if reply == QMessageBox.StandardButton.Yes:
            self.by_upload(activity_id)
            return True
        return False

    def process_file(self, file_path, activity_id=None):
        parser = TcxFileParser()
        df, activity_type = parser.parse_tcx(file_path)
        details = TcxSegmentParser.parse_segments(df, activity_type)
        df = convert_to_utm(df)
        df = calculate_distance(df)

        target = mapActivityTypes(activity_type)
        df, avg_pace, fastest_pace, slowest_pace = calculate_pace(df, target)

        name = os.path.basename(file_path).replace(".tcx", "")

        if not activity_id:
            next_id = self.db.get_next_activity_id()
        else:
            print("activity_id existing: ", activity_id)
            next_id = activity_id

        computed_data = self.compute_data(df, name)
        logger.debug(f"computed_data: {computed_data}")
        computed_data["title"] = generate_activity_title(
            mapActivityTypes(activity_type), computed_data["date"]
        )
        computed_data["activity_id"] = next_id
        computed_data["activity_type"] = activity_type
        computed_data["avg_pace"] = format_hour_minute(avg_pace)
        computed_data["fastest_pace"] = format_hour_minute(fastest_pace)
        computed_data["slowest_pace"] = format_hour_minute(slowest_pace)
        computed_data["pause"] = format_hour_minute(detect_pauses(df))

        segment = get_weather_segment(details)
        weather_data = WeatherService.get_weather(
            segment["latitude"], segment["longitude"], segment["time"]
        )

        if weather_data:
            identifier = {"activity_id": computed_data["activity_id"]}
            weather_data = {**weather_data, **identifier}

        if target == ViewMode.RUN:
            computed_data = self.plot_stats(name, df, computed_data)
            self.process_run(df, computed_data, activity_id)
        elif target == ViewMode.WALK:
            computed_data = self.plot_stats(name, df, computed_data)
            self.process_walk(df, computed_data, activity_id)
        elif target == ViewMode.CYCLE:
            computed_data = self.plot_stats(name, df, computed_data)
            self.process_cycle(df, computed_data, activity_id)
        else:
            return False

        computed_data["id"] = computed_data["activity_id"]

        if activity_id:
            computed_data["activity_id"] = activity_id
            self.db.update_activity(computed_data, details)
            if weather_data:
                self.db.update_weather(weather_data)
        else:
            self.db.insert_activity(computed_data, details)
            if weather_data:
                self.db.insert_weather(weather_data)

    def process_run(self, df, computed_data, activity_id=None):
        avg_steps, total_steps = calculate_steps(df)
        computed_data["avg_steps"] = (
            int(round(avg_steps, 0)) if not np.isnan(avg_steps) else 0
        )
        computed_data["total_steps"] = (
            int(total_steps) if not np.isnan(total_steps) else 0
        )

        if not activity_id:
            self.db.insert_run(computed_data)
        else:
            self.db.update_run(computed_data)

    def process_walk(self, df, computed_data, activity_id=None):
        avg_steps, total_steps = calculate_steps(df)
        computed_data["avg_steps"] = (
            int(round(avg_steps, 0)) if not np.isnan(avg_steps) else 0
        )
        computed_data["total_steps"] = (
            int(total_steps) if not np.isnan(total_steps) else 0
        )
        if not activity_id:
            self.db.insert_walking(computed_data)
        else:
            self.db.update_walking(computed_data)

    def process_cycle(self, df, computed_data, activity_id=None):
        if not activity_id:
            self.db.insert_cycling(computed_data)
        else:
            self.db.update_cycling(computed_data)

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
    def compute_data(df, name=None):
        total_distance = df["Distance"].iloc[-1] if "Distance" in df.columns else 0
        total_time = (
            (
                pd.to_datetime(df["Time"].iloc[-1]) - pd.to_datetime(df["Time"].iloc[0])
            ).total_seconds()
            if "Time" in df.columns
            else 0
        )
        avg_speed = total_distance / (total_time / 3600) if total_time > 0 else 0

        total_calories = df["Calories"].sum() if "Calories" in df.columns else 0

        return {
            "date": pd.to_datetime(df["Time"].iloc[0], utc=True).timestamp(),
            "file_id": name,
            "distance": safe_round(total_distance, 2),
            "duration": total_time,
            "calories": safe_round(total_calories),
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
                safe_round(df["Power"].mean()) if not df["Power"].isnull().all() else 0
            ),
        }


class TcxImportThread(QThread):
    log = pyqtSignal(str)
    finished = pyqtSignal()

    def __init__(self, file_dir, image_path, db_handler: DatabaseHandler):
        super().__init__()
        self.file_dir = file_dir
        self.image_path = image_path
        self.db = db_handler

    def run(self):
        importer = TcxFileImporter(self.file_dir, self.image_path, self.db)
        for filename in os.listdir(self.file_dir):
            if filename.endswith(".tcx"):
                file_path = os.path.join(self.file_dir, filename)
                try:
                    importer.by_file(file_path)
                    self.log.emit(f"Successfully imported {filename}")
                except Exception as e:
                    self.log.emit(f"Failed to import {filename}: {e}")

        self.finished.emit()
