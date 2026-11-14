/** Display formatting for the raw values the Rust commands return. */

const numberFormats = new Map<number, Intl.NumberFormat>();

function numberFormat(digits: number) {
  let format = numberFormats.get(digits);
  if (!format) {
    format = new Intl.NumberFormat(undefined, {
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

const dateTimeFormat = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});
const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

/** Unix timestamp → localised date and time. */
export function formatDateTime(timestamp: number | null | undefined): string {
  return timestamp == null ? "–" : dateTimeFormat.format(new Date(timestamp * 1000));
}

export function formatDate(timestamp: number | null | undefined): string {
  return timestamp == null ? "–" : dateFormat.format(new Date(timestamp * 1000));
}
