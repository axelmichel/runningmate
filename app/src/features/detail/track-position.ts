import { useSyncExternalStore } from "react";

import type { SegmentRow } from "@/bindings/SegmentRow";
import type { TrackPoint } from "@/bindings/TrackPoint";

/**
 * The point hovered on the charts, shared with the map's position dot without re-rendering the
 * charts on every mouse move.
 */
export type HoverStore = {
  get: () => number | null;
  set: (index: number | null) => void;
  subscribe: (listener: () => void) => () => void;
};

export function createHoverStore(): HoverStore {
  let index: number | null = null;
  const listeners = new Set<() => void>();
  return {
    get: () => index,
    set: (next) => {
      if (next === index) return;
      index = next;
      listeners.forEach((listener) => listener());
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

export function useHovered(store: HoverStore): number | null {
  return useSyncExternalStore(store.subscribe, store.get);
}

/** Cumulative distance (km) at the end of each split. */
export function splitEnds(rows: SegmentRow[]): number[] {
  let total = 0;
  return rows.map((row) => (total += row.distance ?? 0));
}

/** Index of the first point at or beyond `km` (the last point past the end). */
export function indexAtKm(points: TrackPoint[], km: number): number {
  const i = points.findIndex((p) => p.distance >= km - 1e-9);
  return i < 0 ? points.length - 1 : i;
}

/** Speed (km/h) from a pace (min/km). */
export function speedFromPace(pace: number | null): number | null {
  return pace && pace > 0 ? 60 / pace : null;
}
