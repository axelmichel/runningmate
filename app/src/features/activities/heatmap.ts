import type { ActivityFilter } from "@/bindings/ActivityFilter";
import type { DayTotal } from "@/bindings/DayTotal";

const DAY_MS = 24 * 3600 * 1000;
const SPAN_DAYS = 365;

export type HeatmapCell = {
  /** Local date, "YYYY-MM-DD". */
  day: string;
  /** Seconds of training that day. */
  duration: number;
  /** 0 (none) to 4 (most), relative to the busiest day in the window. */
  level: 0 | 1 | 2 | 3 | 4;
  /** Outside the 365-day window (padding to full weeks). */
  outside: boolean;
};

function localDay(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** Last day of the window, chosen like `activity_heatmap` in Rust. */
export function heatmapEnd(filter: ActivityFilter, now = new Date()): Date {
  if (filter.maxDate != null) return new Date(filter.maxDate * 1000);
  if (filter.minDate != null) return new Date((filter.minDate + SPAN_DAYS * 86400) * 1000);
  return now;
}

/** Weeks (Monday first) of cells covering the 365 days up to `end`. */
export function heatmapWeeks(days: DayTotal[], end: Date): HeatmapCell[][] {
  const totals = new Map(days.map((d) => [d.day, d.duration]));
  const max = Math.max(0, ...totals.values());
  const endDay = new Date(end.getFullYear(), end.getMonth(), end.getDate());
  const startDay = new Date(endDay.getTime() - (SPAN_DAYS - 1) * DAY_MS);
  // Back up to the Monday of the first week.
  const first = new Date(startDay);
  first.setDate(first.getDate() - ((first.getDay() + 6) % 7));

  const weeks: HeatmapCell[][] = [];
  for (let cursor = new Date(first); cursor <= endDay;) {
    const week: HeatmapCell[] = [];
    for (let i = 0; i < 7; i++) {
      const day = localDay(cursor);
      const duration = totals.get(day) ?? 0;
      const level = duration === 0 || max === 0 ? 0 : Math.min(4, Math.ceil((duration / max) * 4));
      week.push({
        day,
        duration,
        level: level as HeatmapCell["level"],
        outside: cursor < startDay || cursor > endDay,
      });
      cursor.setDate(cursor.getDate() + 1);
    }
    weeks.push(week);
  }
  return weeks;
}
