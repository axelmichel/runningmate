import type { QueryClient } from "@tanstack/react-query";
import type { LatLngTuple } from "leaflet";

import type { FixEdit } from "@/bindings/FixEdit";
import type { FixSample } from "@/bindings/FixSample";

const EARTH_RADIUS = 6_371_008.8;

const radians = (degrees: number) => (degrees * Math.PI) / 180;

/** Great-circle distance in metres. */
export function metres([lat1, lon1]: LatLngTuple, [lat2, lon2]: LatLngTuple): number {
  const dLat = radians(lat2 - lat1);
  const dLon = radians(lon2 - lon1);
  const a =
    Math.sin(dLat / 2) ** 2 +
    Math.cos(radians(lat1)) * Math.cos(radians(lat2)) * Math.sin(dLon / 2) ** 2;
  return 2 * EARTH_RADIUS * Math.asin(Math.min(1, Math.sqrt(a)));
}

/** Length of a line in metres. */
export function pathLength(path: LatLngTuple[]): number {
  let total = 0;
  for (let i = 1; i < path.length; i++) total += metres(path[i - 1], path[i]);
  return total;
}

export function positionOf(sample: FixSample): LatLngTuple | null {
  return sample.lat == null || sample.lon == null ? null : [sample.lat, sample.lon];
}

/** Positions of the samples from `start` to `end` (indices, both included) that have one. */
export function positionsBetween(
  samples: FixSample[],
  start = 0,
  end = samples.length - 1,
): LatLngTuple[] {
  return samples
    .slice(start, end + 1)
    .map(positionOf)
    .filter((p): p is LatLngTuple => p !== null);
}

/**
 * The full route of an edit: the anchor samples' positions (where they have one) around the
 * drawn vertices. The backend moves the samples at `start` and `end` to its first and last point.
 */
export function editPath(
  samples: FixSample[],
  start: number,
  end: number,
  vertices: LatLngTuple[],
): LatLngTuple[] {
  const first = positionOf(samples[start]);
  const last = positionOf(samples[end]);
  return [...(first ? [first] : []), ...vertices, ...(last ? [last] : [])];
}

export function toEdit(
  samples: FixSample[],
  start: number,
  end: number,
  path: LatLngTuple[],
): FixEdit {
  return {
    from: samples[start].time,
    to: samples[end].time,
    path: path.map(([lat, lon]) => [lat, lon]),
  };
}

/**
 * Distance (m) after the edits: the recorded track with each edited stretch replaced by its
 * drawn route. Overlapping edits are not merged; it is a preview, the backend recomputes.
 */
export function editedDistance(samples: FixSample[], edits: FixEdit[]): number {
  let total = pathLength(positionsBetween(samples));
  for (const edit of edits) {
    const start = samples.findIndex((s) => s.time >= edit.from);
    let end = samples.length - 1;
    while (end > 0 && samples[end].time > edit.to) end--;
    if (start >= 0 && end >= start) total -= pathLength(positionsBetween(samples, start, end));
    total += pathLength(edit.path);
  }
  return total;
}

/** Index of the positioned sample nearest to `distanceTo(position)`, within `limit`. */
export function nearestSample(
  samples: FixSample[],
  distanceTo: (position: LatLngTuple) => number,
  limit: number,
): number | null {
  let best: number | null = null;
  let bestDistance = limit;
  samples.forEach((sample, i) => {
    const position = positionOf(sample);
    if (!position) return;
    const d = distanceTo(position);
    if (d <= bestDistance) {
      best = i;
      bestDistance = d;
    }
  });
  return best;
}

/** Everything computed from an activity's track, after it was fixed or reset. */
export function invalidateActivityTrack(queryClient: QueryClient, activityId: number) {
  void queryClient.invalidateQueries({ queryKey: ["activities"] });
  void queryClient.invalidateQueries({ predicate: (query) => query.queryKey[1] === activityId });
}

/** Metres along the track at every sample; samples without a position keep the last value. */
export function cumulativeMetres(samples: FixSample[]): number[] {
  let total = 0;
  let previous: LatLngTuple | null = null;
  return samples.map((sample) => {
    const position = positionOf(sample);
    if (position) {
      if (previous) total += metres(previous, position);
      previous = position;
    }
    return total;
  });
}

/**
 * The stretch to redraw when the track is dragged at sample `index`: from `reach` metres before
 * to `reach` metres after it (the first and last sample at the ends), on positioned samples.
 */
export function stretchAround(
  samples: FixSample[],
  cumulative: number[],
  index: number,
  reach: number,
): [number, number] {
  let start = index;
  while (
    start > 0 &&
    (cumulative[index] - cumulative[start] < reach || !positionOf(samples[start]))
  )
    start--;
  let end = index;
  const last = samples.length - 1;
  while (end < last && (cumulative[end] - cumulative[index] < reach || !positionOf(samples[end])))
    end++;
  return [start, end];
}

/**
 * The vertices with `point` added where it lengthens the route least, between the anchors
 * `first` and `last`. Without both anchors (drawing a route from scratch) it goes at the end.
 */
export function insertVertex(
  first: LatLngTuple | null,
  vertices: LatLngTuple[],
  last: LatLngTuple | null,
  point: LatLngTuple,
): LatLngTuple[] {
  if (!first || !last) return [...vertices, point];
  const route = [first, ...vertices, last];
  let best = 0;
  let bestCost = Infinity;
  for (let k = 0; k < route.length - 1; k++) {
    const cost =
      metres(route[k], point) + metres(point, route[k + 1]) - metres(route[k], route[k + 1]);
    if (cost < bestCost) {
      best = k;
      bestCost = cost;
    }
  }
  return [...vertices.slice(0, best), point, ...vertices.slice(best)];
}

/** The routed line, starting and ending exactly at the waypoints so it joins the track. */
export function joinRouted(waypoints: LatLngTuple[], routed: LatLngTuple[]): LatLngTuple[] {
  if (waypoints.length < 2 || routed.length < 2) return waypoints;
  return [waypoints[0], ...routed, waypoints[waypoints.length - 1]];
}
