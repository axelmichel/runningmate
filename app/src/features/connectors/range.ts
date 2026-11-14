import type { DateRange } from "@/bindings/DateRange";

/** "YYYY-MM-DD" values of the date inputs; empty means open-ended. */
export type RangeInput = { from: string; to: string };

export const OPEN_RANGE: RangeInput = { from: "", to: "" };

/** Local start of the first day and end of the last, as Unix timestamps. */
export function toDateRange(range: RangeInput): DateRange {
  const day = (value: string, end: boolean) => {
    if (!value) return undefined;
    const [y, m, d] = value.split("-").map(Number);
    const date = end ? new Date(y, m - 1, d, 23, 59, 59) : new Date(y, m - 1, d);
    return Math.floor(date.getTime() / 1000);
  };
  const result: DateRange = {};
  const from = day(range.from, false);
  const to = day(range.to, true);
  if (from !== undefined) result.from = from;
  if (to !== undefined) result.to = to;
  return result;
}
