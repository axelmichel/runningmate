import type { ActivityRow } from "@/bindings/ActivityRow";
import type { ActivitySummary } from "@/bindings/ActivitySummary";

/** Weeks in the dashboard's recent period, as Strava's "last 4 weeks". */
export const WEEKS = 4;
const DAY_SECONDS = 24 * 3600;

/** Local date, "YYYY-MM-DD". */
export function localDay(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** Unix start of the recent period: the same time of day `WEEKS` weeks ago. */
export function periodStart(now = new Date()): number {
  return Math.floor(now.getTime() / 1000) - WEEKS * 7 * DAY_SECONDS;
}

/** "YYYY-MM-DD" → that day at 00:00 local time, `null` if invalid. */
export function parseLocalDay(value: string): Date | null {
  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!match) return null;
  const day = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
  return localDay(day) === value ? day : null;
}

/**
 * The day the dashboard looks back from, `WEEKS` weeks further back (-1) or forward (1). `null`
 * means today, which the dashboard follows live; later days aren't offered.
 */
export function stepDay(day: Date | null, direction: -1 | 1, today: Date): Date | null {
  const from = day ?? today;
  const next = new Date(
    from.getFullYear(),
    from.getMonth(),
    from.getDate() + direction * WEEKS * 7,
  );
  return pickDay(next, today);
}

/** A picked day, or `null` for today and later. */
export function pickDay(day: Date, today: Date): Date | null {
  return localDay(day) >= localDay(today) ? null : day;
}

/** The moment the dashboard shows: the end of a picked day, or now. */
export function dashboardMoment(day: Date | null, now: Date): Date {
  return day ? new Date(day.getFullYear(), day.getMonth(), day.getDate(), 23, 59, 59) : now;
}

/** Unix start of the current calendar year, local time. */
export function yearStart(now = new Date()): number {
  return Math.floor(new Date(now.getFullYear(), 0, 1).getTime() / 1000);
}

/** The `WEEKS` calendar weeks (Monday first) up to and including the current one. */
export function calendarWeeks(today = new Date()): Date[][] {
  const monday = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  monday.setDate(monday.getDate() - ((monday.getDay() + 6) % 7) - (WEEKS - 1) * 7);
  return Array.from({ length: WEEKS }, (_, week) =>
    Array.from(
      { length: 7 },
      (_, day) =>
        new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + week * 7 + day),
    ),
  );
}

/** Activities per local day. */
export function byDay(rows: ActivityRow[]): Map<string, ActivityRow[]> {
  const days = new Map<string, ActivityRow[]>();
  for (const row of rows) {
    if (row.date == null) continue;
    const day = localDay(new Date(row.date * 1000));
    days.set(day, [...(days.get(day) ?? []), row]);
  }
  return days;
}

/**
 * Circle diameter for a day's training time, as a share (0–1) of the largest circle: the area
 * grows with the time, relative to the longest day, and a short day still shows as a dot.
 */
export function circleSize(seconds: number, longest: number): number {
  if (seconds <= 0 || longest <= 0) return 0;
  return 0.3 + 0.7 * Math.sqrt(Math.min(1, seconds / longest));
}

/** Totals over `WEEKS` weeks as averages per week. */
export function perWeek(summary: ActivitySummary): ActivitySummary {
  return {
    count: summary.count / WEEKS,
    distance: summary.distance / WEEKS,
    duration: summary.duration / WEEKS,
    elevationGain: summary.elevationGain / WEEKS,
  };
}

/** What the bar chart adds up. */
export type ChartMetric = "distance" | "duration" | "elevation";
/** One bar per calendar week (Monday first) or month. */
export type ChartGrouping = "week" | "month";
/** Bars in the chart. */
export const CHART_BARS = 12;
export const CHART_SPORTS = ["Running", "Cycling", "Walking"] as const;
export type ChartSport = (typeof CHART_SPORTS)[number];

export type ChartBar = Record<ChartSport, number> & {
  /** First day of the week or month. */
  start: Date;
  total: number;
};

/** First days of the `CHART_BARS` weeks or months up to and including the current one. */
export function chartStarts(grouping: ChartGrouping, today = new Date()): Date[] {
  return Array.from({ length: CHART_BARS }, (_, i) => {
    const back = CHART_BARS - 1 - i;
    if (grouping === "month") return new Date(today.getFullYear(), today.getMonth() - back, 1);
    const day = new Date(today.getFullYear(), today.getMonth(), today.getDate());
    return new Date(
      day.getFullYear(),
      day.getMonth(),
      day.getDate() - ((day.getDay() + 6) % 7) - back * 7,
    );
  });
}

/** An activity's contribution: km, hours or metres. */
function amount(row: ActivityRow, metric: ChartMetric): number {
  if (metric === "distance") return row.distance ?? 0;
  if (metric === "duration") return (row.duration ?? 0) / 3600;
  return row.elevationGain ?? 0;
}

/** Totals per sport for each week or month; activities of other types are left out. */
export function chartBars(
  rows: ActivityRow[],
  grouping: ChartGrouping,
  metric: ChartMetric,
  today = new Date(),
): ChartBar[] {
  const starts = chartStarts(grouping, today);
  const bars: ChartBar[] = starts.map((start) => ({
    start,
    total: 0,
    Running: 0,
    Cycling: 0,
    Walking: 0,
  }));
  for (const row of rows) {
    if (row.date == null || !(CHART_SPORTS as readonly string[]).includes(row.viewMode)) continue;
    const time = row.date * 1000;
    // The last bar whose start is not after the activity.
    let index = -1;
    for (let i = 0; i < starts.length && starts[i].getTime() <= time; i++) index = i;
    if (index < 0) continue;
    const value = amount(row, metric);
    bars[index][row.viewMode as ChartSport] += value;
    bars[index].total += value;
  }
  return bars;
}

/**
 * Grid cells (of 4 × 2) for photo `index` of `count`: the first fills the left half, the others
 * share the right half, four of them as a 2 × 2 grid, so together they match the first.
 */
export function tileSpan(index: number, count: number): string {
  if (count === 1) return "col-span-4 row-span-2";
  if (index === 0) return "col-span-2 row-span-2";
  const rest = count - 1;
  if (rest === 1) return "col-span-2 row-span-2";
  if (rest === 2) return "col-span-2";
  // Three: two side by side above, one across below.
  if (rest === 3) return index === 3 ? "col-span-2" : "";
  return "";
}
