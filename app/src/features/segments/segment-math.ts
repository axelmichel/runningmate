import type { QueryClient } from "@tanstack/react-query";
import type { LatLngBoundsExpression, LatLngTuple } from "leaflet";

import type { ActivitySegmentEffort } from "@/bindings/ActivitySegmentEffort";
import type { TrackPoint } from "@/bindings/TrackPoint";
import type { ViewMode } from "@/bindings/ViewMode";
import { formatNumber, formatPace } from "@/lib/format";

/** Shortest segment (km); the backend refuses anything below 100 m. */
export const MIN_SEGMENT_KM = 0.1;

/** Places that earn a trophy. */
export const TROPHY_RANKS = 3;

/** Gold, silver and bronze for the first three places. */
export const TROPHY_COLORS = ["#e5b31b", "#a7adb5", "#c3814a"];

export const SEGMENT_COLOR = "#f37651";
export const START_COLOR = "#f6b48f";
export const FINISH_COLOR = "#e13342";

/** Bounds of a list of positions. */
export function boundsOf(line: LatLngTuple[]): LatLngBoundsExpression {
  const lats = line.map(([lat]) => lat);
  const lons = line.map(([, lon]) => lon);
  return [
    [Math.min(...lats), Math.min(...lons)],
    [Math.max(...lats), Math.max(...lons)],
  ];
}

export type SelectionStats = {
  /** km. */
  distance: number;
  /** m. */
  elevationGain: number;
  /** %. */
  avgGrade: number;
};

/** Distance, climb and average grade between two distances (km) of the series. */
export function selectionStats(points: TrackPoint[], startKm: number, endKm: number) {
  const inside = points.filter((p) => p.distance >= startKm && p.distance <= endKm);
  const distance = endKm - startKm;
  if (inside.length < 2 || distance <= 0) return { distance, elevationGain: 0, avgGrade: 0 };
  let elevationGain = 0;
  for (let i = 1; i < inside.length; i++) {
    elevationGain += Math.max(0, inside[i].elevation - inside[i - 1].elevation);
  }
  const rise = inside[inside.length - 1].elevation - inside[0].elevation;
  return { distance, elevationGain, avgGrade: (rise / (distance * 1000)) * 100 };
}

/** Distance (m) over which the profile's grade is measured, so GPS noise doesn't make spikes. */
const GRADE_WINDOW = 50;

/** Grade (%) at each point, over [`GRADE_WINDOW`] centred on it. */
export function gradeSeries(points: { distance: number; elevation: number }[]): number[] {
  return points.map((p) => {
    let a = 0;
    let b = points.length - 1;
    while (a < points.length - 1 && points[a + 1].distance <= p.distance - GRADE_WINDOW / 2) a++;
    while (b > 0 && points[b - 1].distance >= p.distance + GRADE_WINDOW / 2) b--;
    const run = points[b].distance - points[a].distance;
    return run > 0 ? ((points[b].elevation - points[a].elevation) / run) * 100 : 0;
  });
}

/** Segments where the activity is among the fastest three. */
export function trophyCount(efforts: ActivitySegmentEffort[]): number {
  return new Set(efforts.filter((e) => e.rank <= TROPHY_RANKS).map((e) => e.segmentId)).size;
}

/** Pace (min/km) or, for rides, speed (km/h) over a segment, formatted with or without unit. */
export function formatEffortSpeed(
  view: ViewMode,
  metres: number,
  seconds: number,
  withUnit = true,
): string {
  if (seconds <= 0 || metres <= 0) return "–";
  if (view === "Cycling") {
    const speed = formatNumber(metres / 1000 / (seconds / 3600), 1);
    return withUnit ? `${speed} km/h` : speed;
  }
  const pace = formatPace(seconds / 60 / (metres / 1000));
  return withUnit ? `${pace} /km` : pace;
}

export function formatGrade(percent: number): string {
  return `${formatNumber(percent, 1)} %`;
}

export function formatSegmentDistance(metres: number): string {
  return metres < 1000 ? `${formatNumber(metres)} m` : `${formatNumber(metres / 1000, 2)} km`;
}

/** Everything that shows segments or passes over them. */
export function invalidateSegments(queryClient: QueryClient) {
  void queryClient.invalidateQueries({ queryKey: ["routeSegments"] });
  void queryClient.invalidateQueries({ queryKey: ["routeSegment"] });
  void queryClient.invalidateQueries({ queryKey: ["segmentEfforts"] });
}
