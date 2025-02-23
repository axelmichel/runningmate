from datetime import datetime

import requests

from utils.logger import logger


class WeatherService:
    @staticmethod
    def get_weather(lat: float, lon: float, date: str) -> dict | None:
        """
        :param lat: Latitude of the location.
        :param lon: Longitude of the location.
        :param date: Date in the format "YYYY-MM-DD".
        :return: dict | None: Weather data containing temperature, precipitation, and wind speed,
                         or None if data is unavailable.
        """
        today = datetime.now().strftime("%Y-%m-%d")

        if date < today:
            return WeatherService.get_historical_weather(lat, lon, date)
        else:
            return WeatherService.get_current_weather(lat, lon)

    @staticmethod
    def get_current_weather(lat: float, lon: float) -> dict | None:
        """
        Fetches current weather data from Open-Meteo API.

        :param lat: Latitude of the location.
        :param lon: Longitude of the location.
        :return: dict | None: Current weather data including temperature, wind speed, precipitation,
                         or None if data is unavailable.
        """
        url = "https://api.open-meteo.com/v1/forecast"

        logger.info(f"Fetching current weather data at {lat}, {lon}")
        params = {
            "latitude": lat,
            "longitude": lon,
            "current": [
                "temperature_2m",
                "windspeed_10m",
                "precipitation",
                "weather_code",
            ],
            "timezone": "auto",
        }

        response = requests.get(url, params=params)
        data = response.json()

        if "current" in data:
            logger.info("Successfully received current weather data")
            return {
                "date": datetime.now().strftime("%Y-%m-%d"),
                "max_temp": data["current"]["temperature_2m"],  # Current temp as max
                "min_temp": data["current"]["temperature_2m"],  # Same as above
                "precipitation": data["current"]["precipitation"],
                "max_wind_speed": data["current"]["windspeed_10m"],
                "weather_code": data["current"]["weather_code"],
                "source": "current",
            }
        logger.warning(f"No weather data: {data}")
        return None

    @staticmethod
    def get_historical_weather(lat: float, lon: float, date: str) -> dict | None:
        """
        Fetches historical weather data from Open-Meteo API.

        :param lat: Latitude of the location.
        :param lon: Longitude of the location.
        :param date: Date in the format "YYYY-MM-DD".
        :return: dict | None: Historical weather data including temperature, wind speed, and precipitation,
                         or None if data is unavailable.
        """
        url = "https://archive-api.open-meteo.com/v1/archive"

        logger.info(f"Fetching historical weather data for {date} at {lat}, {lon}")
        params = {
            "latitude": lat,
            "longitude": lon,
            "start_date": date,
            "end_date": date,  # Single date
            "daily": [
                "temperature_2m_max",
                "temperature_2m_min",
                "precipitation_sum",
                "windspeed_10m_max",
                "weather_code",
            ],
            "timezone": "auto",
        }

        response = requests.get(url, params=params)
        data = response.json()

        if "daily" in data:
            logger.info(f"Successfully received historical weather data for {date}")
            return {
                "max_temp": data["daily"]["temperature_2m_max"][0],
                "min_temp": data["daily"]["temperature_2m_min"][0],
                "precipitation": data["daily"]["precipitation_sum"][0],
                "max_wind_speed": data["daily"]["windspeed_10m_max"][0],
                "weather_code": data["daily"]["weather_code"][0],
                "source": "historical",
            }
        logger.warning(f"No historical weather data for {date}: {data}")
        return None
