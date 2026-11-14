import {
  Cloud,
  CloudDrizzle,
  CloudFog,
  CloudHail,
  CloudLightning,
  CloudRain,
  CloudSnow,
  CloudSun,
  Sun,
  ThermometerSun,
} from "lucide-react";

/** Icon groups follow `weather_code_icon` in `utils/translations.py`. */
export function WeatherIcon({ code, className }: { code: number | null; className?: string }) {
  if (code == null) return <ThermometerSun className={className} />;
  if (code <= 1) return <Sun className={className} />;
  if (code === 2) return <CloudSun className={className} />;
  if (code === 3 || code === 48) return <Cloud className={className} />;
  if (code === 45) return <CloudFog className={className} />;
  if (code >= 51 && code <= 57) return <CloudDrizzle className={className} />;
  if (code === 77) return <CloudHail className={className} />;
  if ((code >= 71 && code <= 75) || code === 85 || code === 86) {
    return <CloudSnow className={className} />;
  }
  if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82)) {
    return <CloudRain className={className} />;
  }
  if (code >= 95) return <CloudLightning className={className} />;
  return <ThermometerSun className={className} />;
}
