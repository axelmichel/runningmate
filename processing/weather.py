from datetime import datetime

import requests


def get_weather(lat, lon, date):
    today = datetime.now().strftime("%Y-%m-%d")

    if date < today:
        return get_historical_weather(lat, lon, date)
    else:
        return get_current_weather(lat, lon)

def get_current_weather(lat, lon):
    url = "https://api.open-meteo.com/v1/forecast"

    params = {
        "latitude": lat,
        "longitude": lon,
        "current": ["temperature_2m", "windspeed_10m", "precipitation", "weathercode"],
        "timezone": "auto"
    }

    response = requests.get(url, params=params)
    data = response.json()

    if "current" in data:
        weather = {
            "date": datetime.now().strftime("%Y-%m-%d"),
            "max_temp": data["current"]["temperature_2m"],  # Current temp used as "max" for today
            "min_temp": data["current"]["temperature_2m"],  # Same as above
            "precipitation": data["current"]["precipitation"],
            "max_wind_speed": data["current"]["windspeed_10m"],
            "source": "current"
        }
        return weather
    return None

def get_historical_weather(lat, lon, date):
    url = "https://archive-api.open-meteo.com/v1/archive"

    params = {
        "latitude": lat,
        "longitude": lon,
        "start_date": date,
        "end_date": date,  # Single date
        "daily": ["temperature_2m_max", "temperature_2m_min", "precipitation_sum", "windspeed_10m_max"],
        "timezone": "auto"
    }

    response = requests.get(url, params=params)
    data = response.json()

    if "daily" in data:
        weather = {
            "max_temp": data["daily"]["temperature_2m_max"][0],
            "min_temp": data["daily"]["temperature_2m_min"][0],
            "precipitation": data["daily"]["precipitation_sum"][0],
            "max_wind_speed": data["daily"]["windspeed_10m_max"][0]
        }
        return weather
    return None