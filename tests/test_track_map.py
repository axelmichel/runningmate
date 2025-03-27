from unittest.mock import MagicMock, patch

import pandas as pd
import pytest

from processing.track_map import TrackMap


@pytest.fixture
def dummy_df():
    return pd.DataFrame({
        "Latitude": [50.0, 50.001, 50.002],
        "Longitude": [8.0, 8.001, 8.002],
        "HeartRate": [120, 125, 130],
        "CleanPace": [5.0, 5.2, 5.1]
    })


@pytest.fixture
def track_map(tmp_path, test_db):
    return TrackMap(
        file_path=str(tmp_path),
        image_path=str(tmp_path),
        db_handler=test_db,
        activity_id=1
    )


def test_validate_dataframe_success(track_map, dummy_df):
    track_map.df = dummy_df
    track_map._validate_dataframe()


def test_validate_dataframe_missing_column(track_map):
    track_map.df = pd.DataFrame({
        "Latitude": [1],
        "Longitude": [1],
        "HeartRate": [90]
        # Missing CleanPace
    })
    with pytest.raises(ValueError, match="missing required columns"):
        track_map._validate_dataframe()


def test_haversine_calculation():
    p1 = (50.0, 8.0)
    p2 = (50.001, 8.001)
    dist = TrackMap._haversine(p1, p2)
    assert 0 < dist < 0.2


@patch("processing.track_map.ActivityData")
def test_create_map_track(mock_activity_data_cls, dummy_df, tmp_path, test_db):
    mock_data = MagicMock()
    mock_data.get_activity_df.return_value = dummy_df
    mock_data.get_activity_type.return_value = "running"
    mock_data.get_activity_identifier.return_value = "test"
    mock_data.save_activity_map.return_value = "track_map_saved"

    mock_activity_data_cls.return_value = mock_data

    # TrackMap created AFTER mocking is in place
    track_map = TrackMap(
        file_path=str(tmp_path),
        image_path=str(tmp_path),
        db_handler=test_db,
        activity_id=1
    )

    result = track_map.create_map("track")
    assert result == "track_map_saved"


@patch("processing.track_map.ActivityData")
def test_create_map_heart_rate(mock_activity_data_cls, dummy_df, tmp_path, test_db):
    mock_data = MagicMock()
    mock_data.get_activity_df.return_value = dummy_df
    mock_data.get_activity_type.return_value = "running"
    mock_data.get_activity_identifier.return_value = "test"
    mock_data.save_activity_map.return_value = "hr_map_saved"

    mock_activity_data_cls.return_value = mock_data

    track_map = TrackMap(
        file_path=str(tmp_path),
        image_path=str(tmp_path),
        db_handler=test_db,
        activity_id=1
    )

    result = track_map.create_map("heart_rate")
    assert result == "hr_map_saved"