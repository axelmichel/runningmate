/** Display formatting for the raw values the Rust commands return, in the app language. */

let locale: string | undefined;
const numberFormats = new Map<number, Intl.NumberFormat>();
let dateTimeFormat: Intl.DateTimeFormat;
let dateFormat: Intl.DateTimeFormat;
let timeFormat: Intl.DateTimeFormat;

/** Switches number and date formatting to the app language ("en", "de", "fr"). */
export function setFormatLocale(language: string | undefined) {
  locale = language;
  numberFormats.clear();
  dateTimeFormat = new Intl.DateTimeFormat(locale, { dateStyle: "medium", timeStyle: "short" });
  dateFormat = new Intl.DateTimeFormat(locale, { dateStyle: "medium" });
  timeFormat = new Intl.DateTimeFormat(locale, { timeStyle: "medium" });
}
setFormatLocale(undefined);

function numberFormat(digits: number) {
  let format = numberFormats.get(digits);
  if (!format) {
    format = new Intl.NumberFormat(locale, {
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    });
    numberFormats.set(digits, format);
  }
  return format;
}

export function formatNumber(value: number | null | undefined, digits = 0): string {
  return value == null || Number.isNaN(value) ? "–" : numberFormat(digits).format(value);
}

/** Seconds → "H:MM:SS". */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null) return "–";
  const total = Math.max(0, Math.round(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

/** "H:MM:SS" (or "MM:SS") → seconds, `null` if invalid. */
export function parseDuration(text: string): number | null {
  const parts = text.trim().split(":").map(Number);
  if (parts.length < 2 || parts.length > 3 || parts.some((p) => !Number.isInteger(p) || p < 0))
    return null;
  const [h, m, s] = parts.length === 3 ? parts : [0, ...parts];
  return m < 60 && s < 60 ? h * 3600 + m * 60 + s : null;
}

/** Seconds → "123 h 05 min", for totals. */
export function formatHours(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  return `${formatNumber(Math.floor(minutes / 60))} h ${String(minutes % 60).padStart(2, "0")} min`;
}

/**
 * Pace in decimal min/km → "M:SS". Rounds to whole seconds first, so 5.999 becomes "6:00"
 * instead of the Python app's "05:60".
 */
export function formatPace(minutesPerKm: number | null | undefined): string {
  if (minutesPerKm == null || !Number.isFinite(minutesPerKm) || minutesPerKm <= 0) return "–";
  const total = Math.round(minutesPerKm * 60);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
}

/** Stored "MM:SS" paces/pauses → "M:SS"; repairs "08:60" style values written by Python. */
export function formatMinSec(value: string | null | undefined): string {
  const match = value?.match(/^(\d+):(\d+)$/);
  if (!match) return value || "–";
  return formatPace(Number(match[1]) + Number(match[2]) / 60).replace("–", "0:00");
}

/** Unix timestamp → localised date and time. */
export function formatDateTime(timestamp: number | null | undefined): string {
  return timestamp == null ? "–" : dateTimeFormat.format(new Date(timestamp * 1000));
}

export function formatDate(timestamp: number | null | undefined): string {
  return timestamp == null ? "–" : dateFormat.format(new Date(timestamp * 1000));
}

/** Time of day with seconds, e.g. "14:27:05". */
export function formatTime(timestamp: number | null | undefined): string {
  return timestamp == null ? "–" : timeFormat.format(new Date(timestamp * 1000));
}
