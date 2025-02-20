import pandas as pd
import pytest

from processing.system_settings import ViewMode
from processing.tcx_segment_parser import TcxSegmentParser


@pytest.fixture
def sample_dataframe():
    """Provides a sample DataFrame for testing."""
    data = {
        "Time": ["2024-02-20T12:00:00Z", "2024-02-20T12:01:00Z"],
        "Latitude": [37.7749, 37.7750],
        "Longitude": [-122.4194, -122.4195],
        "Elevation": [10.0, 15.0],
        "HeartRate": [150, 155],
        "Power": [200, 210],
        "Speed": [3.0, 3.5],
        "Steps": [80, 85],
    }
    return pd.DataFrame(data)


def test_get_segment_data():
    """Tests that _get_segment_data initializes properly."""
    segment_data = TcxSegmentParser._get_segment_data()
    assert isinstance(segment_data, dict)
    assert segment_data["heart_rate"] == []
    assert segment_data["seg_distance"] == 0.0


def test_compute_averages():
    """Tests the computation of averages."""
    segment_data = {
        "heart_rate": [140, 150, 160],
        "power": [180, 190, 200],
        "speed": [2.5, 2.8, 3.0],
        "steps": [75, 80, 85],
        "elevation": [5, 10, 15],
    }
    averages = TcxSegmentParser._compute_averages(segment_data)
    assert averages["avg_heart_rate"] == 150
    assert averages["avg_power"] == 190
    assert averages["avg_speed"] == pytest.approx(2.766, rel=1e-3)
    assert averages["avg_steps"] == 80
    assert averages["avg_elevation"] == 10


def test_store_segment():
    """Tests storing a segment."""
    segments = []
    segment_data = TcxSegmentParser._get_segment_data()
    segment_data["seg_latitude"] = 37.7749
    segment_data["seg_longitude"] = -122.4194
    segment_data["seg_distance"] = 1.2
    TcxSegmentParser._store_segment(
        segments, segment_data, "2024-02-20T12:00:00Z", ViewMode.RUN
    )

    assert len(segments) == 1
    assert segments[0]["seg_latitude"] == 37.7749
    assert segments[0]["seg_distance"] == 1.2
    assert segments[0]["seg_time_end"] == "2024-02-20T12:00:00Z"


def test_get_segment_length():
    """Tests the segment length determination."""
    assert TcxSegmentParser._get_segment_length(ViewMode.CYCLE) == 5.0
    assert TcxSegmentParser._get_segment_length(ViewMode.RUN) == 1.0


def test_parse_segments(sample_dataframe):
    """Tests segment parsing."""
    result = TcxSegmentParser.parse_segments(sample_dataframe, "Running")
    assert isinstance(result, pd.DataFrame)
    assert not result.empty
    assert "seg_latitude" in result.columns
    assert "seg_distance" in result.columns
