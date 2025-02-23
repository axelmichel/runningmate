import numpy as np
import pandas as pd
from pyproj import Transformer

from processing.system_settings import ViewMode


def convert_to_utm(df):
    """Converts GPS coordinates (Lat, Lon) into UTM coordinates for distortion removal."""
    if "Latitude" not in df or "Longitude" not in df:
        raise ValueError("Missing Latitude or Longitude columns in DataFrame")

    # Ensure no NaN values before projection
    df = df.dropna(subset=["Latitude", "Longitude"])

    # Compute UTM Zone dynamically based on mean longitude
    lon_mean = df["Longitude"].mean()
    utm_zone = int((lon_mean + 180) / 6) + 1
    proj_string = f"+proj=utm +zone={utm_zone} +datum=WGS84 +units=m +no_defs"

    # Transformer for projection
    transformer = Transformer.from_crs("EPSG:4326", proj_string, always_xy=True)

    # Apply transformation
    df[["X", "Y"]] = df.apply(
        lambda row: pd.Series(transformer.transform(row["Longitude"], row["Latitude"])),
        axis=1,
    )

    return df


def calculate_pace(df, activity: ViewMode):
    """Calculates pace (min/km) for each segment and applies different filtering based on activity type."""
    # Set pace thresholds based on activity type
    if activity == ViewMode.RUN:
        min_pace, max_pace, min_dist = 3, 12, 0.5
    elif activity == ViewMode.CYCLE:
        min_pace, max_pace, min_dist = 0.5, 6, 5
    elif activity == ViewMode.WALK:
        min_pace, max_pace, min_dist = 8, 25, 0.2
    else:
        min_pace, max_pace, min_dist = 2, 20, 0.5

    # Avoid extreme values by filtering based on min distance
    df["DistDiff"] = df["DistDiff"].replace(0, np.nan)

    df["Pace"] = np.where(
        df["DistDiff"].notna() & (df["DistDiff"] > min_dist),
        (df["TimeDiff"] / df["DistDiff"]) * 16.6667,  # ✅ Correct conversion factor
        np.nan,
    )

    df["Pace"] = df["Pace"].where(
        (df["Pace"] >= min_pace) & (df["Pace"] <= max_pace), np.nan
    )

    df["Pace"] = df["Pace"].replace([np.inf, -np.inf], np.nan)

    # Compute valid average, fastest, and slowest pace
    avg_pace = df["Pace"].mean(skipna=True)
    fastest_pace = df["Pace"].quantile(0.05, interpolation="nearest")  # 5th percentile
    slowest_pace = df["Pace"].quantile(0.95, interpolation="nearest")  # 95th percentile

    return df, avg_pace, fastest_pace, slowest_pace


def detect_pauses(df, threshold=10):
    """Detects pauses if there is a significant gap in time (default: 10 seconds)."""
    df["PauseDetected"] = df["TimeDiff"] > threshold
    total_pause_time = df.loc[df["PauseDetected"], "TimeDiff"].sum() / 60
    return total_pause_time


def calculate_distance(df):
    """Calculates the cumulative distance in kilometers with decimal precision."""

    df["X"] = df["X"].astype(float)
    df["Y"] = df["Y"].astype(float)

    diffs_x = np.diff(df["X"], prepend=df["X"].iloc[0])
    diffs_y = np.diff(df["Y"], prepend=df["Y"].iloc[0])

    distances = np.sqrt(diffs_x**2 + diffs_y**2) / 1000  # Convert meters to km
    cumulative_distance = distances.cumsum().astype(float)  # ✅ Force float type

    df["Distance"] = cumulative_distance

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
