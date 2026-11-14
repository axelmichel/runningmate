import type { ActivityFilter } from "@/bindings/ActivityFilter";
import { t } from "@/i18n";
import { formatDate, formatNumber } from "@/lib/format";

/** Form values as typed; empty strings mean "not set". */
export type Draft = Record<
  | "minDate"
  | "maxDate"
  | "minDistance"
  | "maxDistance"
  | "minDuration"
  | "maxDuration"
  | "minElevation"
  | "maxElevation",
  string
>;

function toDateInput(timestamp: number | undefined): string {
  if (timestamp == null) return "";
  const date = new Date(timestamp * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** "YYYY-MM-DD" → Unix timestamp of local midnight (or the end of that day). */
function fromDateInput(value: string, endOfDay: boolean): number | undefined {
  if (!value) return undefined;
  const [y, m, d] = value.split("-").map(Number);
  const date = endOfDay ? new Date(y, m - 1, d, 23, 59, 59) : new Date(y, m - 1, d);
  return Math.floor(date.getTime() / 1000);
}

const num = (value: string, scale = 1): number | undefined =>
  value.trim() === "" || Number.isNaN(Number(value)) ? undefined : Number(value) * scale;

const str = (value: number | undefined, scale = 1) =>
  value == null ? "" : String(Math.round((value / scale) * 100) / 100);

export function toDraft(filter: ActivityFilter): Draft {
  return {
    minDate: toDateInput(filter.minDate),
    maxDate: toDateInput(filter.maxDate),
    minDistance: str(filter.minDistance),
    maxDistance: str(filter.maxDistance),
    minDuration: str(filter.minDuration, 60),
    maxDuration: str(filter.maxDuration, 60),
    minElevation: str(filter.minElevation),
    maxElevation: str(filter.maxElevation),
  };
}

/** Builds the filter; a max below its min is raised to the min, as the Python dialog did. */
export function draftToFilter(draft: Draft, searchText?: string): ActivityFilter {
  const range = (min: number | undefined, max: number | undefined) =>
    [min, min != null && max != null && max < min ? min : max] as const;
  const [minDate, maxDate] = range(
    fromDateInput(draft.minDate, false),
    fromDateInput(draft.maxDate, true),
  );
  const [minDistance, maxDistance] = range(num(draft.minDistance), num(draft.maxDistance));
  const [minDuration, maxDuration] = range(num(draft.minDuration, 60), num(draft.maxDuration, 60));
  const [minElevation, maxElevation] = range(num(draft.minElevation), num(draft.maxElevation));
  const filter: ActivityFilter = {
    searchText,
    minDate,
    maxDate,
    minDistance,
    maxDistance,
    minDuration: minDuration == null ? undefined : Math.round(minDuration),
    maxDuration: maxDuration == null ? undefined : Math.round(maxDuration),
    minElevation,
    maxElevation,
  };
  return Object.fromEntries(
    Object.entries(filter).filter(([, value]) => value !== undefined && value !== ""),
  ) as ActivityFilter;
}

/** Short labels for the active range filters, shown next to the search box. */
export function filterLabels(filter: ActivityFilter): string[] {
  const range = (
    label: string,
    min: number | undefined,
    max: number | undefined,
    show: (value: number) => string,
  ) => {
    if (min == null && max == null) return [];
    if (min != null && max != null) return [`${label} ${show(min)}–${show(max)}`];
    return [min != null ? `${label} ≥ ${show(min)}` : `${label} ≤ ${show(max!)}`];
  };
  return [
    ...range(t("Date"), filter.minDate, filter.maxDate, formatDate),
    ...range(
      t("Distance"),
      filter.minDistance,
      filter.maxDistance,
      (v) => `${formatNumber(v, 1)} km`,
    ),
    ...range(
      t("Duration"),
      filter.minDuration,
      filter.maxDuration,
      (v) => `${Math.round(v / 60)} min`,
    ),
    ...range(
      t("Elevation"),
      filter.minElevation,
      filter.maxElevation,
      (v) => `${formatNumber(v)} m`,
    ),
  ];
}
