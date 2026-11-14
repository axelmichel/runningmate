import { t, tk } from "@/i18n";

/** WMO weather code labels, from `utils/translations.py`. */
const LABELS: Record<number, string> = {
  0: tk("Clear sky"),
  1: tk("Mainly clear"),
  2: tk("Partly cloudy"),
  3: tk("Overcast"),
  45: tk("Fog"),
  48: tk("Depositing rime fog"),
  51: tk("Light drizzle"),
  53: tk("Moderate drizzle"),
  55: tk("Dense drizzle"),
  56: tk("Light freezing drizzle"),
  57: tk("Dense freezing drizzle"),
  61: tk("Slight rain"),
  63: tk("Moderate rain"),
  65: tk("Heavy rain"),
  66: tk("Light freezing rain"),
  67: tk("Dense freezing rain"),
  71: tk("Slight snow fall"),
  73: tk("Moderate snow fall"),
  75: tk("Heavy snow fall"),
  77: tk("Snow grains"),
  80: tk("Slight rain showers"),
  81: tk("Moderate rain showers"),
  82: tk("Violent rain showers"),
  85: tk("Slight snow showers"),
  86: tk("Heavy snow showers"),
  95: tk("Thunderstorm"),
  96: tk("Thunderstorm with slight hail"),
  99: tk("Thunderstorm with heavy hail"),
};

export function weatherLabel(code: number | null): string {
  return t((code != null && LABELS[code]) || tk("Unknown"));
}
