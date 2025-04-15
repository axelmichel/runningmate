import gettext
import locale
import os

from processing.system_settings import get_settings_value
from utils.app_mode import is_dark_mode
from utils.resource_path import resource_path

lang, _ = locale.getlocale()
default_lang = lang.split("_")[0] if lang else "en"
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
    user_preferred_lang = get_settings_value("language", default_lang)

    lang_code = (
        lang_code or user_preferred_lang
    )  # Use provided language or system default
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
    return _(wmo.get(code, _("unknown"))).title()


def weather_code_icon(code: int) -> str | None:  # noqa: C901
    """
    Get the icon name based on the weather code.
    :param code: int, WMO weather code
    :return: str, icon name
    """

    icon_folder = "light" if is_dark_mode() else "dark"

    if code in (0, 1):
        return resource_path(f"icons/{icon_folder}/sun-line.svg")
    elif code == 2:
        return resource_path(f"icons/{icon_folder}/sun-cloudy-line.svg")
    elif code in (3, 48):
        return resource_path(f"icons/{icon_folder}/cloudy-2-line.svg")
    elif code == 45:
        return resource_path(f"icons/{icon_folder}/foggy-line.svg")
    elif code in (51, 53, 55, 56, 57, 61, 63, 65, 66, 67):
        return resource_path(f"icons/{icon_folder}/drizzle-line.svg")
    elif code in (71, 73, 75):
        return resource_path(f"icons/{icon_folder}/snowy-line.svg")
    elif code == 77:
        return resource_path(f"icons/{icon_folder}/hail-line.svg")
    elif code in (80, 81, 82):
        return resource_path(f"icons/{icon_folder}/showers-line.svg")
    elif code in (85, 86):
        return resource_path(f"icons/{icon_folder}/snowy-line.svg")
    elif code in (95, 96, 99):
        return resource_path(f"icons/{icon_folder}/thunderstorms-line.svg")
    else:
        return None
