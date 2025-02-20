import pandas as pd
import pytest

from processing.data_processing import (
    calculate_distance,
    calculate_pace,
    calculate_steps,
    convert_to_utm,
    detect_pauses,
)
from processing.system_settings import ViewMode


@pytest.fixture
def sample_df():
    """Creates a sample DataFrame with GPS coordinates, timestamps, and other required fields."""
    data = {
        "Time": pd.to_datetime(
            ["2024-02-20T10:00:00", "2024-02-20T10:01:00", "2024-02-20T10:02:00"]
        ),
        "Latitude": [52.5200, 52.5201, 52.5202],
        "Longitude": [13.4050, 13.4051, 13.4052],
        "Elevation": [35, 36, 37],
        "Steps": [80, 85, 90],
    }
    return pd.DataFrame(data)


def test_convert_to_utm(sample_df):
    """Test converting GPS coordinates to UTM and ensuring correct data transformation."""
    df = convert_to_utm(sample_df.copy())
    assert "X" in df and "Y" in df, "UTM conversion should add X and Y columns"
    assert (
        df["X"].isna().sum() == 0 and df["Y"].isna().sum() == 0
    ), "UTM coordinates should not be NaN"


def test_convert_to_utm_missing_columns():
    """Ensure function raises error when missing required GPS fields."""
    df = pd.DataFrame({"Latitude": [52.52, 52.53]})  # Missing Longitude
    with pytest.raises(ValueError, match="Missing Latitude or Longitude columns"):
        convert_to_utm(df)


def test_calculate_pace_with_invalid_data():
    """Ensure invalid pace values are correctly filtered out."""
    df = pd.DataFrame(
        {
            "Time": pd.to_datetime(["2024-02-20T10:00:00", "2024-02-20T10:01:00"]),
            "Distance": [0, 0],
            "TimeDiff": [0, 60],
            "DistDiff": [0, 0],
        }
    )  # Zero distance movement

    df, avg_pace, fastest_pace, slowest_pace = calculate_pace(df, ViewMode.RUN)

    assert pd.isna(df["Pace"]).all(), "Pace should be NaN for zero distance movement"
    assert (
        pd.isna(avg_pace) and pd.isna(fastest_pace) and pd.isna(slowest_pace)
    ), "Pace stats should be NaN"


def test_detect_pauses(sample_df):
    """Test pause detection based on time gaps."""
    df = sample_df.copy()
    df["TimeDiff"] = [0, 5, 15]  # Simulate time differences

    total_pause_time = detect_pauses(df, threshold=10)

    assert total_pause_time > 0, "There should be a detected pause"
    assert total_pause_time == 0.25, "Pause should be correctly summed in minutes"


def test_calculate_distance(sample_df):
    """Test cumulative distance calculation in kilometers."""
    df = convert_to_utm(sample_df.copy())
    df = calculate_distance(df)

    assert "Distance" in df, "Distance column should exist"
    assert df["Distance"].iloc[-1] > 0, "Cumulative distance should be positive"


def test_calculate_steps(sample_df):
    """Test step count calculations for total and average cadence."""
    avg_steps, total_steps = calculate_steps(sample_df.copy())

    assert avg_steps > 0, "Average step cadence should be positive"
    assert total_steps > 0, "Total steps should be positive"
