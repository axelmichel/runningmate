from pyproj import Transformer
import numpy as np
import pandas as pd

from processing.system_settings import ViewMode


def convert_to_utm(df):
    """Converts GPS coordinates (Lat, Lon) into UTM coordinates for distortion removal."""
    if "Latitude" not in df or "Longitude" not in df:
        raise ValueError("Missing Latitude or Longitude columns in DataFrame")

    # Ensure no NaN values before projection
    df = df.dropna(subset=["Latitude", "Longitude"])

    # Compute UTM Zone dynamically based on mean longitude
    lat_mean = df["Latitude"].mean()
    lon_mean = df["Longitude"].mean()
    utm_zone = int((lon_mean + 180) / 6) + 1
    proj_string = f"+proj=utm +zone={utm_zone} +datum=WGS84 +units=m +no_defs"

    # Transformer for projection
    transformer = Transformer.from_crs("EPSG:4326", proj_string, always_xy=True)

    # Apply transformation
    df[["X", "Y"]] = df.apply(lambda row: pd.Series(transformer.transform(row["Longitude"], row["Latitude"])), axis=1)

    return df


def calculate_pace(df, activity:ViewMode):
    """Calculates pace (min/km) for each segment and applies different filtering based on activity type."""
    df["Time"] = pd.to_datetime(df["Time"])
    df["TimeDiff"] = df["Time"].diff().dt.total_seconds()
    df["DistDiff"] = df["Distance"].diff()

    # Set pace thresholds based on activity type
    if activity == ViewMode.RUN:
        min_pace, max_pace, min_dist = 3, 12, 0.003  # 2-15 min/km, min distance 3m
    elif activity == ViewMode.CYCLE:
        min_pace, max_pace, min_dist = 0.5, 6, 0.01  # 30 sec - 6 min/km, min distance 10m
    elif activity == ViewMode.WALK:
        min_pace, max_pace, min_dist = 8, 25, 0.001  # 8-25 min/km, min distance 1m
    else:  # Unknown activity type
        min_pace, max_pace, min_dist = 2, 20, 0.001  # Default reasonable values

    # Avoid extreme values by filtering based on min distance
    df["Pace"] = np.where(df["DistDiff"] > min_dist, (df["TimeDiff"] / 60) / df["DistDiff"], np.nan)

    # Replace infinite values with NaN
    df["Pace"] = df["Pace"].replace([np.inf, -np.inf], np.nan)

    # Apply pace limits based on activity type
    df["Pace"] = df["Pace"].where((df["Pace"] >= min_pace) & (df["Pace"] <= max_pace), np.nan)

    # Compute valid average, fastest, and slowest pace
    avg_pace = df["Pace"].mean(skipna=True)
    fastest_pace = df["Pace"].min(skipna=True)
    slowest_pace = df["Pace"].max(skipna=True)

    return df, avg_pace, fastest_pace, slowest_pace


def detect_pauses(df, threshold=10):
    """Detects pauses if there is a significant gap in time (default: 10 seconds)."""
    df["PauseDetected"] = df["TimeDiff"] > threshold
    total_pause_time = df.loc[df["PauseDetected"], "TimeDiff"].sum() / 60
    return total_pause_time


def calculate_distance(df):
    """Calculates the cumulative distance in kilometers along the route."""
    df["Distance"] = np.sqrt(np.diff(df["X"], prepend=df["X"].iloc[0]) ** 2 +
                             np.diff(df["Y"], prepend=df["Y"].iloc[0]) ** 2).cumsum() / 1000  # Umwandlung in km
    return df


def calculate_steps(df):
    """Corrects step calculations for accurate cadence and total step count."""
    df["Time"] = pd.to_datetime(df["Time"])
    df["TimeDiff"] = df["Time"].diff().dt.total_seconds()

    # If cadence is per foot, multiply by 2 to get total steps per minute
    df["Steps"] = df["Steps"] * 2

    # Compute total steps by integrating over time
    df["StepContribution"] = (df["Steps"] * df["TimeDiff"]) / 60
    total_steps = df["StepContribution"].sum()

    # Compute average step cadence (SPM)
    avg_steps = df["Steps"].mean()

    return avg_steps, total_steps
