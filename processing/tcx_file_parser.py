import xml.etree.ElementTree as ET
from typing import Dict, Optional, Tuple

import numpy as np
import pandas as pd
from geopy.distance import geodesic
from pyproj import Transformer

from processing.system_settings import ViewMode, mapActivityTypes


class TcxFileParser:
    def __init__(self, user_params: Optional[Dict[str, Dict[str, float]]] = None):
        """
        Initialize the TCX Parser with user-specific parameters for different activities.

        :param user_params: Dictionary containing parameters for different activity types.
        Example:
        {
            "Cycling": {"rider_weight": 70, "bike_weight": 8, "rolling_resistance": 0.004, "air_density": 1.225,
                        "drag_area": 0.27, "wind_speed": 0},
            "Running": {"runner_weight": 70, "shoe_resistance": 0.02},
            "Walking": {"walker_weight": 70, "shoe_resistance": 0.03}
        }
        """
        self.user_params: Dict[str, Dict[str, float]] = (
            user_params
            if user_params
            else {
                "Cycling": {
                    "rider_weight": 70,
                    "bike_weight": 8,
                    "rolling_resistance": 0.004,
                    "air_density": 1.225,
                    "drag_area": 0.27,
                    "wind_speed": 0,
                },
                "Running": {"runner_weight": 70, "shoe_resistance": 0.02},
                "Walking": {"walker_weight": 70, "shoe_resistance": 0.03},
            }
        )
        self.g: float = 9.81  # Gravity (m/s²)

    @staticmethod
    def extract_activity_type(root: ET.Element, namespaces: Dict[str, str]) -> str:
        """
        Extracts the activity type (Running, Biking, Walking) from a TCX file.

        :param root: XML root element of the TCX file.
        :param namespaces: Dictionary containing XML namespace mappings.
        :return: Activity type as a string (e.g., "Biking", "Running", "Walking").
        """
        activity = root.find(".//tcx:Activity", namespaces)
        return (
            activity.attrib.get("Sport", "Unknown")
            if activity is not None
            else "Unknown"
        )

    def parse_tcx(self, file_path: str) -> Tuple[pd.DataFrame, str]:
        """
        Parses a TCX file and extracts GPS data, elevation, heart rate, and power. If power is missing,
        it is estimated based on the activity type.

        :param file_path: Path to the TCX file.
        :return: A tuple containing:
            - A Pandas DataFrame with parsed data.
            - A string representing the detected activity type.
        """
        tree = ET.parse(file_path)
        root = tree.getroot()
        namespaces = {
            "tcx": "http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2"
        }
        trackpoints = []
        heart_rates = []

        for trackpoint in root.findall(".//tcx:Trackpoint", namespaces):
            lat = trackpoint.find(".//tcx:LatitudeDegrees", namespaces)
            lon = trackpoint.find(".//tcx:LongitudeDegrees", namespaces)
            ele = trackpoint.find(".//tcx:AltitudeMeters", namespaces)
            time = trackpoint.find(".//tcx:Time", namespaces)
            heart_rate = trackpoint.find(".//tcx:HeartRateBpm/tcx:Value", namespaces)
            extensions = trackpoint.find(".//tcx:Extensions", namespaces)

            if heart_rate is not None:
                heart_rates.append(int(heart_rate.text))

            steps, power = None, None
            if extensions is not None:
                tpx = extensions.find(
                    ".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}TPX"
                )
                if tpx is not None:
                    steps = tpx.find(
                        ".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}RunCadence"
                    )
                    power = tpx.find(
                        ".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}Watts"
                    )

            if (
                lat is not None
                and lon is not None
                and ele is not None
                and time is not None
            ):
                trackpoints.append(
                    (
                        time.text,
                        float(lat.text),
                        float(lon.text),
                        float(ele.text),
                        int(steps.text) if steps is not None else None,
                        int(power.text) if power is not None else None,
                    )
                )

        df = pd.DataFrame(
            trackpoints,
            columns=["Time", "Latitude", "Longitude", "Elevation", "Steps", "Power"],
        )
        df["HeartRate"] = (
            pd.Series(heart_rates) if heart_rates else pd.Series(dtype="float")
        )
        df["Time"] = pd.to_datetime(df["Time"], utc=True)  # ✅ Ensure proper timestamps
        df["TimeDiff"] = df["Time"].diff().dt.total_seconds()

        if (df["TimeDiff"] == 1.0).sum() > len(df) * 0.8:
            df["TimeDiff"] = df["TimeDiff"].rolling(5, min_periods=1).mean()

        df["DistDiff"] = df.apply(self.calculate_distance, axis=1, df=df)
        df["DistDiff"] = df["DistDiff"].replace(0, np.nan)  # ✅ Replace 0s with NaN
        df["DistDiff"] = df["DistDiff"].interpolate()

        df["Speed"] = df["DistDiff"] / df["TimeDiff"]

        activity_type = self.extract_activity_type(root, namespaces)
        activity_group = mapActivityTypes(activity_type)

        df = self._map_power(df, activity_group)
        df = self._map_pace(df, activity_group)
        df = self._map_distance(df)
        df["Calories"] = df.apply(
            lambda row: self._estimate_calories(row, activity_group), axis=1
        )
        return df, activity_type

    def _map_power(self, df, activity_group):
        """
        Maps power values to the DataFrame based on the activity type.

        :param df: The DataFrame containing power data.
        :param activity_group: The detected activity group (ViewMode Enum: CYCLE, RUN, WALK).
        """
        df["EstimatedPower"] = df.apply(
            lambda row: self.estimate_power(row, activity_group), axis=1
        )
        df["Power"] = df["Power"].astype(object)
        df["Power"] = df["Power"].apply(
            lambda x: np.nan if x in [None, "None", ""] else x
        )
        df["Power"] = df["Power"].fillna(df["EstimatedPower"])
        df["Power"] = pd.to_numeric(df["Power"], errors="coerce").astype(float)
        df["Power"] = df["Power"].infer_objects(copy=False)
        df.drop(columns=["EstimatedPower"], inplace=True)
        return df

    @staticmethod
    def _map_distance(df):
        """Calculates the cumulative distance in kilometers using projected coordinates."""
        df = df.dropna(subset=["Latitude", "Longitude"])
        df["Latitude"] = df["Latitude"].astype(float)
        df["Longitude"] = df["Longitude"].astype(float)

        # Vectorized transformation
        transformer = Transformer.from_crs("EPSG:4326", "EPSG:32633", always_xy=True)
        x, y = transformer.transform(df["Longitude"].tolist(), df["Latitude"].tolist())
        df["X"] = x
        df["Y"] = y

        diffs_x = np.diff(df["X"], prepend=df["X"].iloc[0])
        diffs_y = np.diff(df["Y"], prepend=df["Y"].iloc[0])

        distances = np.sqrt(diffs_x**2 + diffs_y**2) / 1000
        df["DistanceInKm"] = distances.cumsum()
        return df

    @staticmethod
    def _map_pace(df, activity_group):
        """
        Calculates pace (min/km) for each segment and applies different filtering based on activity type.
        :param df: The DataFrame containing distance and time data.
        :param activity_group: The detected activity group (ViewMode Enum: CYCLE, RUN, WALK).
        """
        # Set pace thresholds based on activity type
        if activity_group == ViewMode.RUN:
            min_pace, max_pace, min_dist = 3, 12, 0.5
        elif activity_group == ViewMode.CYCLE:
            min_pace, max_pace, min_dist = 0.5, 6, 5
        elif activity_group == ViewMode.WALK:
            min_pace, max_pace, min_dist = 8, 25, 0.2
        else:
            min_pace, max_pace, min_dist = 2, 20, 0.5

        # Avoid extreme values by filtering based on min distance
        df["CleanDistDiff"] = df["DistDiff"].replace(0, np.nan)

        df["CleanPace"] = np.where(
            df["CleanDistDiff"].notna() & (df["CleanDistDiff"] > min_dist),
            (df["TimeDiff"] / df["CleanDistDiff"])
            * 16.6667,  # ✅ Correct conversion factor
            np.nan,
        )

        df["CleanPace"] = df["CleanPace"].where(
            (df["CleanPace"] >= min_pace) & (df["CleanPace"] <= max_pace), np.nan
        )

        df["CleanPace"] = df["CleanPace"].replace([np.inf, -np.inf], np.nan)
        return df

    @staticmethod
    def _estimate_calories(row: pd.Series, activity_type: ViewMode) -> float:
        """
        Estimates calories burned based on power output and efficiency.

        :param row: A row from the DataFrame containing power and time data.
        :param activity_type: The detected activity type (ViewMode Enum: CYCLE, RUN, WALK).
        :return: Estimated calories burned (float).
        """
        power = row["Power"]
        time_seconds = row["TimeDiff"]

        if (
            not isinstance(power, (int, float))
            or np.isnan(power)
            or time_seconds is None
            or np.isnan(time_seconds)
        ):
            return 0.0  # Default if data is missing

        # Define efficiency based on activity type
        if activity_type is ViewMode.CYCLE:
            efficiency = 0.25  # 25% efficiency for cycling
        elif activity_type is ViewMode.RUN:
            efficiency = 0.20  # 20% efficiency for running
        elif activity_type is ViewMode.WALK:
            efficiency = 0.20  # 20% efficiency for walking
        else:
            efficiency = 0.20  # Default efficiency

        # Convert Watts × Seconds to Calories
        joules = power * time_seconds
        calories = joules / (efficiency * 4184)  # 1 kcal = 4184 Joules

        return max(0.0, calories)  # Ensure no negative calories

    @staticmethod
    def calculate_distance(row: pd.Series, df: pd.DataFrame) -> float:
        """
        Computes the distance between consecutive GPS points using geodesic distance.

        :param row: A row from the DataFrame containing latitude and longitude.
        :param df: The DataFrame containing trackpoint data.
        :return: Distance in meters (float).
        """
        try:
            row_index = int(row.name)
        except (ValueError, TypeError):
            return 0.0  # Default to 0 if conversion fails

        if row_index == 0:
            return 0.0  # Ensure float return type

        prev_row = df.iloc[row_index - 1]  # Use the previous row safely
        return geodesic(
            (prev_row["Latitude"], prev_row["Longitude"]),
            (row["Latitude"], row["Longitude"]),
        ).meters

    def estimate_power(self, row: pd.Series, activity_type: ViewMode) -> float:
        """
        Estimates power output based on activity type.

        :param row: A row from the DataFrame containing speed and elevation data.
        :param activity_type: The detected activity type (ViewMode Enum: CYCLE, RUN, or WALK).
        :return: Estimated power in watts.
        """
        power_value = row.get("Power")
        if isinstance(power_value, (int, float)):
            if not np.isnan(power_value):
                return power_value  # Use existing power if it's a valid number

        v: float = row["Speed"] if isinstance(row["Speed"], (int, float)) else 0.0

        if activity_type is ViewMode.CYCLE:
            return self._estimate_ride(v)
        elif activity_type is ViewMode.RUN:
            return self._estimate_run(v)
        elif activity_type is ViewMode.WALK:
            return self._estimate_walk(v)

        return 0.0

    def _estimate_ride(self, v: float) -> float:
        """
        Estimates power output for cycling based on speed and elevation.

        :return: Estimated power in watts.
        """
        params = self.user_params["Cycling"]
        total_mass = params["rider_weight"] + params["bike_weight"]
        rolling_resistance = params["rolling_resistance"]
        air_density = params["air_density"]
        drag_area = params["drag_area"]
        wind_speed = params["wind_speed"]

        p_roll: float = rolling_resistance * total_mass * self.g * v
        p_aero: float = 0.5 * air_density * drag_area * (v + wind_speed) ** 3
        return max(0.0, p_roll + p_aero)

    def _estimate_run(self, v: float):
        """
        Estimates power output for running based on speed and elevation.

        :return: Estimated power in watts.
        """
        params = self.user_params["Running"]
        total_mass: float = params["runner_weight"]
        shoe_resistance: float = params["shoe_resistance"]
        return max(0.0, shoe_resistance * total_mass * self.g * v)

    def _estimate_walk(self, v: float):
        """
        Estimates power output for walking based on speed and elevation.

        :return: Estimated power in watts.
        """
        params = self.user_params["Walking"]
        total_mass: float = params["walker_weight"]
        shoe_resistance: float = params["shoe_resistance"]
        return max(0.0, shoe_resistance * total_mass * self.g * v)
