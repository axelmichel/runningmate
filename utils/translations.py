import gettext
import locale
import os

default_lang, _ = locale.getlocale()

LOCALE_DIR = os.path.join(os.path.dirname(__file__), "../locales")

WMO_UNKNOWN = 99999

wmo = {
    0: "clear sky",
    1: "mainly clear",
    2: "partly cloudy",
    3: "overcast",
    45: "fog",
    48: "depositing rime fog",
    51: "light drizzle",
    53: "moderate drizzle",
    55: "dense drizzle",
    56: "light freezing drizzle",
    57: "dense freezing drizzle",
    61: "slight rain",
    63: "moderate rain",
    65: "heavy rain",
    66: "light freezing rain",
    67: "dense freezing rain",
    71: "slight snow fall",
    73: "moderate snow fall",
    75: "heavy snow fall",
    77: "snow grains",
    80: "slight rain showers",
    81: "moderate rain showers",
    82: "violent rain showers",
    85: "slight snow shower",
    86: "heavy snow showers",
    95: "thunderstorm",
    96: "thunderstorm with slight hail",
    99: "thunderstorm with heavy hail",
    WMO_UNKNOWN: "unknown",
}


def set_language(lang_code=None):
    """
    Set the application language dynamically.

    If translation files are not found, it falls back to a string manipulation:
    - Replaces underscores (_) with spaces
    - Converts to title case (each word starts with uppercase)

    :param lang_code: str, optional
        The language code (e.g., "en", "de", "fr"). If not provided, defaults to `default_lang`.
    :return: function
        A gettext translation function or a fallback function.
    """
    lang_code = lang_code or default_lang  # Use provided language or system default
    try:
        lang = gettext.translation(
            "messages", localedir=LOCALE_DIR, languages=[lang_code]
        )
        lang.install()
        return lang.gettext  # Return the gettext function
    except FileNotFoundError:

        def fallback_translation(s):
            return s.replace("_", " ").title() if "_" in s else s

        return fallback_translation


_ = set_language()


def translate_weather_code(code: int) -> str:
    """
    Converts the weather code to a human-readable string.
    :param code: int, WMO weather code
    :return: str, translated weather description
    """
    return _(wmo.get(code, _("unknown")).title())
