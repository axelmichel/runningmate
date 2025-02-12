import numpy as np
import pandas as pd
from translations import _

from processing.system_settings import ViewMode


def format_hour_minute(pace):
    """Converts a pace value in min/km to MM:SS format."""
    if np.isnan(pace) or pace is None:
        return "00:00"
    minutes = int(pace)
    seconds = int(round((pace - minutes) * 60))
    return f"{minutes:02d}:{seconds:02d}"


def compute_run_statistics(df, base_name, avg_steps, total_steps, avg_pace, fastest_pace, slowest_pace,
                           total_pause_time, activity_type):
    """Computes various running statistics from the DataFrame and returns them as a dictionary."""

    # Compute core metrics
    total_distance = df["Distance"].iloc[-1]
    total_time = pd.to_datetime(df["Time"].iloc[-1]) - pd.to_datetime(df["Time"].iloc[0])
    avg_speed = total_distance / (total_time.total_seconds() / 3600)  # Convert to km/h
    avg_power = df["Power"].mean()

    # Build the statistics dictionary
    data = {
        "Date": base_name.split("_")[0].replace("-", "."),
        "Start Time (HH:mm)": pd.to_datetime(df["Time"].iloc[0]).strftime("%H:%M"),
        "Distance (km)": round(total_distance, 2),
        "Time (HH:mm:ss)": str(total_time).split()[2] if total_time else "00:00:00",
        "Elevation Gain (m)": int(round(df["Elevation"].diff().clip(lower=0).sum(), 0)),
        "Average Speed (km/h)": round(avg_speed, 2),
        "Average Steps (SPM)": int(round(avg_steps, 0)) if not np.isnan(avg_steps) else 0,
        "Total Steps": int(total_steps) if not np.isnan(total_steps) else 0,
        "Average Power (Watts)": int(round(avg_power, 0)) if not np.isnan(avg_power) else 0,
        "Average Heart Rate (BPM)": int(round(df["HeartRate"].mean(), 0)) if not df["HeartRate"].isnull().all() else 0,
        "Average Pace (min/km)": format_hour_minute(avg_pace),  # Convert to MM:SS format
        "Fastest Kilometer (min/km)": format_hour_minute(fastest_pace),  # Convert to MM:SS format
        "Slowest Kilometer (min/km)": format_hour_minute(slowest_pace),  # Convert to MM:SS format
        "Pause": format_hour_minute(total_pause_time),
        "Activity": activity_type
    }

    return data


def generate_activity_title(activity_type: ViewMode, timestamp: float) -> str:
    """
    Generate a title like "Run in the Afternoon" or "Cycling at Night" using pandas.

    :param activity_type: The type of activity (e.g., "Running", "Cycling").
    :param timestamp: Unix timestamp (seconds).
    :return: Formatted activity title.
    """
    dt = pd.to_datetime(timestamp, unit="s", utc=True)
    hour = dt.hour  # Extract the hour (0-23)

    if 5 <= hour < 12:
        time_of_day = _("in the Morning")
    elif 12 <= hour < 17:
        time_of_day = _("in the Afternoon")
    elif 17 <= hour < 21:
        time_of_day = _("in the Evening")
    else:
        time_of_day = _("at Night")

    label = "Activity"
    if(activity_type == ViewMode.CYCLE):
        label = _("Ride")
    elif(activity_type == ViewMode.WALK):
        label = _("Walk")
    elif(activity_type == ViewMode.RUN):
        label = _("Run")

    return f"{label} {time_of_day}"


def compute_run_db_data(df, date, month, year, avg_steps, total_steps, avg_pace, fastest_pace, slowest_pace,
                        total_pause_time, activity_type):
    """Computes running statistics and prepares them for database insertion."""

    # Compute core metrics
    total_distance = df["Distance"].iloc[-1] if "Distance" in df.columns else 0
    total_time = pd.to_datetime(df["Time"].iloc[-1]) - pd.to_datetime(
        df["Time"].iloc[0]) if "Time" in df.columns else pd.Timedelta(0)
    avg_speed = total_distance / (total_time.total_seconds() / 3600) if total_time.total_seconds() > 0 else 0
    avg_power = df["Power"].mean() if "Power" in df.columns else 0
    avg_heart_rate = df["HeartRate"].mean() if "HeartRate" in df.columns else 0

    # Compute Elevation Gain
    elevation_gain = int(round(df["Elevation"].diff().clip(lower=0).sum(), 0)) if "Elevation" in df.columns else 0

    # Formatting
    formatted_data = (
        date.split("_")[0].replace("-", "."),
        year,
        month,
        pd.to_datetime(df["Time"].iloc[0]).strftime("%H:%M") if "Time" in df.columns else "00:00",
        round(total_distance, 2),
        str(total_time).split()[2] if total_time else "00:00:00",
        elevation_gain,
        round(avg_speed, 2),
        int(round(avg_steps, 0)) if not np.isnan(avg_steps) else 0,
        int(total_steps) if not np.isnan(total_steps) else 0,
        int(round(avg_power, 0)) if not np.isnan(avg_power) else 0,
        int(round(avg_heart_rate, 0)) if not np.isnan(avg_heart_rate) else 0,
        format_hour_minute(avg_pace),
        format_hour_minute(fastest_pace),
        format_hour_minute(slowest_pace),
        format_hour_minute(total_pause_time),
        activity_type
    )

    return formatted_data
