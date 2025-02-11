import os

import pytest
import xml.etree.ElementTree as ET
import pandas as pd

from processing.parse_tcx import extract_activity_type, parse_tcx


@pytest.fixture
def sample_tcx():
    """Provides a minimal valid TCX sample with a Running activity and a few trackpoints."""
    return """<?xml version="1.0" encoding="UTF-8"?>
    <TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
        <Activities>
            <Activity Sport="Running">
                <Id>2024-02-20T10:00:00Z</Id>
                <Lap>
                    <Track>
                        <Trackpoint>
                            <Time>2024-02-20T10:01:00Z</Time>
                            <Position>
                                <LatitudeDegrees>52.5200</LatitudeDegrees>
                                <LongitudeDegrees>13.4050</LongitudeDegrees>
                            </Position>
                            <AltitudeMeters>35.0</AltitudeMeters>
                            <HeartRateBpm>
                                <Value>150</Value>
                            </HeartRateBpm>
                            <Extensions>
                                <TPX xmlns="http://www.garmin.com/xmlschemas/ActivityExtension/v2">
                                    <RunCadence>85</RunCadence>
                                    <Watts>250</Watts>
                                </TPX>
                            </Extensions>
                        </Trackpoint>
                    </Track>
                </Lap>
            </Activity>
        </Activities>
    </TrainingCenterDatabase>"""


@pytest.fixture
def sample_tcx_no_activity():
    """Provides a TCX file with missing activity type to test fallback behavior."""
    return """<?xml version="1.0" encoding="UTF-8"?>
    <TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
        <Activities>
            <Activity>
                <Id>2024-02-20T10:00:00Z</Id>
            </Activity>
        </Activities>
    </TrainingCenterDatabase>"""


@pytest.fixture
def sample_tcx_no_heart_rate():
    """Provides a TCX file with missing heart rate data."""
    return """<?xml version="1.0" encoding="UTF-8"?>
    <TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
        <Activities>
            <Activity Sport="Running">
                <Lap>
                    <Track>
                        <Trackpoint>
                            <Time>2024-02-20T10:01:00Z</Time>
                            <Position>
                                <LatitudeDegrees>52.5200</LatitudeDegrees>
                                <LongitudeDegrees>13.4050</LongitudeDegrees>
                            </Position>
                            <AltitudeMeters>35.0</AltitudeMeters>
                        </Trackpoint>
                    </Track>
                </Lap>
            </Activity>
        </Activities>
    </TrainingCenterDatabase>"""


def test_extract_activity_type(sample_tcx):
    """Test extracting the activity type from a TCX file."""
    root = ET.ElementTree(ET.fromstring(sample_tcx)).getroot()
    namespaces = {'tcx': 'http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2'}
    assert extract_activity_type(root, namespaces) == "Running"


def test_extract_activity_type_missing(sample_tcx_no_activity):
    """Test fallback when activity type is missing in TCX file."""
    root = ET.ElementTree(ET.fromstring(sample_tcx_no_activity)).getroot()
    namespaces = {'tcx': 'http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2'}
    assert extract_activity_type(root, namespaces) == "Unknown"


def test_parse_tcx(sample_tcx, tmp_path):
    """Test parsing a TCX file and extracting data correctly."""
    tcx_file = tmp_path / "test.tcx"
    tcx_file.write_text(sample_tcx)

    df, activity_type = parse_tcx(str(tcx_file))

    assert activity_type == "Running"
    assert len(df) == 1  # ✅ Only one trackpoint in sample
    assert df.iloc[0]["Latitude"] == 52.5200
    assert df.iloc[0]["Longitude"] == 13.4050
    assert df.iloc[0]["Elevation"] == 35.0
    assert df.iloc[0]["HeartRate"] == 150
    assert df.iloc[0]["Steps"] == 85
    assert df.iloc[0]["Power"] == 250


def test_parse_tcx_missing_heart_rate(sample_tcx_no_heart_rate, tmp_path):
    """Test parsing a TCX file where heart rate data is missing."""
    tcx_file = tmp_path / "test_no_hr.tcx"
    tcx_file.write_text(sample_tcx_no_heart_rate)

    df, activity_type = parse_tcx(str(tcx_file))

    assert activity_type == "Running"
    assert len(df) == 1  # ✅ Only one trackpoint in sample
    assert df.iloc[0]["HeartRate"] is None or pd.isna(df.iloc[0]["HeartRate"])  # ✅ Should handle missing HR gracefully


def test_parse_tcx_no_data(tmp_path):
    """Test parsing an empty TCX file."""
    tcx_file = tmp_path / "empty.tcx"
    tcx_file.write_text("<TrainingCenterDatabase></TrainingCenterDatabase>")

    df, activity_type = parse_tcx(str(tcx_file))

    assert activity_type == "Unknown"
    assert df.empty  # ✅ DataFrame should be empty


@pytest.fixture(scope="module", autouse=True)
def cleanup_test_files():
    """Cleanup test-generated TCX files."""
    yield
    test_files = ["test.tcx", "test_no_hr.tcx", "empty.tcx"]
    for file in test_files:
        if os.path.exists(file):
            os.remove(file)
