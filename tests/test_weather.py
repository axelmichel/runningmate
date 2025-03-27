from datetime import datetime
from unittest.mock import patch

import pytest

from processing.weather import WeatherService


@pytest.fixture
def sample_current_weather():
    """Fixture to return a sample current weather API response."""
    return {
        "current": {
            "temperature_2m": 15.3,
            "windspeed_10m": 12.4,
            "precipitation": 0.8,
            "weather_code": 2,
            "wind_direction_10m": "N",
        }
    }


@pytest.fixture
def sample_historical_weather():
    """Fixture to return a sample historical weather API response."""
    return {
        "daily": {
            "temperature_2m_max": [20.5],
            "temperature_2m_min": [10.2],
            "precipitation_sum": [5.1],
            "windspeed_10m_max": [18.7],
            "wind_direction_10m_dominant": ["N"],
            "weather_code": [3],
        }
    }


@patch("processing.weather.requests.get")
def test_get_current_weather_success(mock_get, sample_current_weather):
    """Test fetching current weather successfully with mocked API response."""
    mock_get.return_value.json.return_value = sample_current_weather

    result = WeatherService.get_current_weather(48.609229, -3.836006)

    expected = {
        "date": datetime.now().strftime("%Y-%m-%d"),
        "max_temp": 15.3,
        "min_temp": 15.3,
        "precipitation": 0.8,
        "max_wind_speed": 12.4,
        "weather_code": 2,
        "wind_direction": "N",
        "source": "current",
    }

    assert result == expected
    mock_get.assert_called_once()


@patch("processing.weather.requests.get")
def test_get_current_weather_no_data(mock_get):
    """Test current weather returns None when API returns no data."""
    mock_get.return_value.json.return_value = {}

    result = WeatherService.get_current_weather(48.609229, -3.836006)

    assert result is None
    mock_get.assert_called_once()


@patch("processing.weather.requests.get")
def test_get_historical_weather_success(mock_get, sample_historical_weather):
    """Test fetching historical weather successfully with mocked API response."""
    mock_get.return_value.json.return_value = sample_historical_weather

    result = WeatherService.get_historical_weather(48.609229, -3.836006, "2024-02-15")

    expected = {
        "max_temp": 20.5,
        "min_temp": 10.2,
        "precipitation": 5.1,
        "max_wind_speed": 18.7,
        "weather_code": 3,
        "wind_direction": "N",
        "source": "historical",
    }

    assert result == expected
    mock_get.assert_called_once()


@patch("processing.weather.requests.get")
def test_get_historical_weather_no_data(mock_get):
    """Test historical weather returns None when API returns no data."""
    mock_get.return_value.json.return_value = {}

    result = WeatherService.get_historical_weather(48.609229, -3.836006, "2024-02-15")

    assert result is None
    mock_get.assert_called_once()


@patch("processing.weather.WeatherService.get_historical_weather")
@patch("processing.weather.WeatherService.get_current_weather")
def test_get_weather_decides_correctly(mock_get_current, mock_get_historical):
    """Test get_weather correctly chooses between historical and current weather."""
    today = datetime.now().strftime("%Y-%m-%d")
    mock_get_current.return_value = {"mocked": "current"}
    mock_get_historical.return_value = {"mocked": "historical"}

    # Test historical date
    result = WeatherService.get_weather(48.609229, -3.836006, "2024-02-15")
    assert result == {"mocked": "historical"}
    mock_get_historical.assert_called_once()
    mock_get_current.assert_not_called()

    # Reset mock call count
    mock_get_historical.reset_mock()
    mock_get_current.reset_mock()

    # Test current date
    result = WeatherService.get_weather(48.609229, -3.836006, today)
    assert result == {"mocked": "current"}
    mock_get_current.assert_called_once()
    mock_get_historical.assert_not_called()
