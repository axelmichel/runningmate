import { toDateRange, type RangeInput } from "@/features/connectors/range";

/** Quick choices for the period, in months. */
export const PRESET_MONTHS = [1, 3, 6, 12] as const;

/** Months shown when the page opens. */
export const DEFAULT_MONTHS = 3;

function isoDay(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** The last `months` months up to and including `today`. */
export function lastMonths(months: number, today = new Date()): RangeInput {
  const from = new Date(today.getFullYear(), today.getMonth() - months, today.getDate() + 1);
  return { from: isoDay(from), to: isoDay(today) };
}

/** Unix seconds from the start of the first day to the end of the last; `null` if incomplete. */
export function periodBounds(range: RangeInput): { from: number; to: number } | null {
  const { from, to } = toDateRange(range);
  return from !== undefined && to !== undefined && from <= to ? { from, to } : null;
}
