import type { LatLngTuple } from "leaflet";

import type { TrackPoint } from "@/bindings/TrackPoint";

/** Positions where each full kilometre (5 km for rides) is reached. */
export function kilometreMarkers(points: TrackPoint[], every: number) {
  const markers: { km: number; position: LatLngTuple }[] = [];
  let next = every;
  for (const p of points) {
    if (p.distance >= next) {
      markers.push({ km: next, position: [p.latitude, p.longitude] });
      next += every;
    }
  }
  return markers;
}

/** Arrow spacings (km) to choose from, shortest first. */
const ARROW_STEPS = [0.1, 0.2, 0.25, 0.5, 1, 2, 2.5, 5];
/** About this many arrows along a track. */
const ARROWS = 40;

/** Compass bearing (degrees clockwise from north) from `a` to `b`, as drawn on the map. */
export function bearing(a: TrackPoint, b: TrackPoint): number {
  const east = (b.longitude - a.longitude) * Math.cos((a.latitude * Math.PI) / 180);
  const north = b.latitude - a.latitude;
  return ((Math.atan2(east, north) * 180) / Math.PI + 360) % 360;
}

/**
 * Arrows along the track in the direction it was recorded, spaced so there are about
 * [`ARROWS`] of them, halfway between the kilometre markers rather than on them. `from` and
 * `to` (km) limit them to a stretch.
 */
export function directionArrows(
  points: TrackPoint[],
  from = 0,
  to = Number.POSITIVE_INFINITY,
): { position: LatLngTuple; bearing: number }[] {
  if (points.length < 2) return [];
  const total = Math.min(to, points[points.length - 1].distance) - from;
  if (total <= 0) return [];
  const step = ARROW_STEPS.find((s) => total / s <= ARROWS) ?? ARROW_STEPS[ARROW_STEPS.length - 1];
  const arrows: { position: LatLngTuple; bearing: number }[] = [];
  let next = from + step / 2;
  for (let i = 1; i < points.length; i++) {
    const [a, b] = [points[i - 1], points[i]];
    if (b.distance > to) break;
    while (b.distance >= next) {
      if (next >= from && b.distance > a.distance) {
        const t = (next - a.distance) / (b.distance - a.distance);
        arrows.push({
          position: [
            a.latitude + (b.latitude - a.latitude) * t,
            a.longitude + (b.longitude - a.longitude) * t,
          ],
          bearing: bearing(a, b),
        });
      }
      next += step;
    }
  }
  return arrows;
}

/**
 * The point a click picks: among points within `tolerance` (screen pixels, from `pixels`), the
 * first pass over the spot from index `from` on, and in it the closest point. A run of
 * consecutive points near the click is one pass; a track that comes back to the spot later
 * makes a second pass, which is only taken if the first one is before `from`. `null` if the
 * click is too far from the track.
 */
export function snapToPass(pixels: number[], tolerance: number, from = 0): number | null {
  let best: number | null = null;
  for (let i = Math.max(0, from); i < pixels.length; i++) {
    if (pixels[i] <= tolerance) {
      if (best === null || pixels[i] < pixels[best]) best = i;
    } else if (best !== null) {
      break;
    }
  }
  return best;
}
