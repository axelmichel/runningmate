import type { QueryClient } from "@tanstack/react-query";

/** `[lowest, highest]` bpm of zones 1–5 for upper bounds, with zone 1 starting at `floor`. */
export function zoneRanges(floor: number, upper: number[]): [number, number][] {
  return upper.map((bound, i) => [i === 0 ? floor : upper[i - 1] + 1, bound]);
}

/** Zone 1 of custom bounds starts at half of the zone 5 bound, as `users::resolve_zones` does. */
export function customFloor(upper: number[]): number {
  return Math.floor(upper[4] / 2);
}

/**
 * Refetches everything drawn from the zones. Saving the zones or the profile can change them:
 * the derived methods follow the maximum and resting heart rate.
 */
export function invalidateZones(queryClient: QueryClient) {
  for (const key of ["zones", "zonePreview", "heartRateTrend", "health", "training"]) {
    void queryClient.invalidateQueries({ queryKey: [key] });
  }
}
