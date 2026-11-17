import { useQueries } from "@tanstack/react-query";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import type { EffectScore } from "@/bindings/EffectScore";
import type { SegmentRow } from "@/bindings/SegmentRow";
import type { TrackPoint } from "@/bindings/TrackPoint";
import type { TrainingEffect } from "@/bindings/TrainingEffect";
import type { ViewMode } from "@/bindings/ViewMode";
import type { ZoneTime } from "@/bindings/ZoneTime";
import { api } from "@/lib/api";
import { formatDate, formatDateTime } from "@/lib/format";

import { compareColor, type CompareSelection, type Stretch } from "./compare-math";

/** A compared activity with everything the charts need, as far as it has loaded. */
export type CompareEntry = {
  id: number;
  /** Key of its values in chart rows. */
  key: string;
  color: string;
  /** The date (with the time when two share a day), for legends and tooltips. */
  label: string;
  detail: ActivityDetail | undefined;
  /** Couldn't be loaded, e.g. deleted since. */
  missing: boolean;
  view: ViewMode;
  /** The segment pass to limit it to, when comparing segment passes. */
  stretch: Stretch | null;
  /** `undefined` while loading, `null` without a track. */
  series: TrackPoint[] | null | undefined;
  splits: SegmentRow[] | undefined;
  effect: TrainingEffect | null | undefined;
  score: EffectScore | undefined;
  zones: ZoneTime[] | null | undefined;
};

/**
 * Loads the selected activities with the query keys of the detail window, so both share their
 * caches.
 */
export function useCompareData(selection: CompareSelection): CompareEntry[] {
  const ids = selection.items.map((item) => item.id);
  const each = <T>(key: string, fn: (id: number) => Promise<T>, staleTime?: number) =>
    ids.map((id) => ({ queryKey: [key, id], queryFn: () => fn(id), staleTime }));
  const details = useQueries({ queries: each("activity", api.getActivity) });
  // Parsed from the archived file, so kept for the session like `useSeries`.
  const series = useQueries({ queries: each("series", api.activitySeries, Infinity) });
  const splits = useQueries({ queries: each("segments", api.activitySegments) });
  const effects = useQueries({ queries: each("trainingEffect", api.activityTrainingEffect) });
  const scores = useQueries({ queries: each("effectScore", api.activityEffectScore) });
  const zones = useQueries({ queries: each("zones", api.activityHeartRateZones) });

  const days = details.map((d) => formatDate(d.data?.date));
  return selection.items.map((item, i) => {
    const detail = details[i].data;
    const sameDay = days.filter((day) => day === days[i]).length > 1;
    return {
      id: item.id,
      key: `a${item.id}`,
      color: compareColor(item.slot),
      label: detail ? (sameDay ? formatDateTime(detail.date) : days[i]) : "…",
      detail,
      missing: details[i].isError,
      view: detail?.viewMode ?? "All",
      stretch: selection.segment?.stretches[item.id] ?? null,
      series: series[i].data,
      splits: splits[i].data,
      effect: effects[i].data,
      score: scores[i].data,
      zones: zones[i].data,
    };
  });
}
