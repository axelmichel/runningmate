/** Zone colours from dark to light, after `ui/widget_heart_rate_zones.py`. */
export const ZONE_COLORS = ["#35193e", "#701f57", "#ad1759", "#f37651", "#f6b48f"];

/** One colour band of a vertical gradient: `from`–`to` are offsets 0 (top) to 1 (bottom). */
export type GradientBand = { zone: number; from: number; to: number };

/**
 * Bands of a top-to-bottom gradient over heart rates `min`–`max` that colour each part of the
 * line by its zone. `bounds` are the zones' upper bounds (zone 1 first); heart rates below zone 1
 * count as zone 1 and above zone 5 as zone 5, as in `analysis/zones.rs`. Empty when the line is
 * flat (a gradient over a zero-height box draws nothing).
 */
export function zoneGradient(min: number, max: number, bounds: number[]): GradientBand[] {
  if (!(max > min)) return [];
  const offset = (bpm: number) => Math.min(1, Math.max(0, (max - bpm) / (max - min)));
  const bands: GradientBand[] = [];
  for (let zone = 5; zone >= 1; zone--) {
    const upper = zone === 5 ? Infinity : bounds[zone - 1];
    const lower = zone === 1 ? -Infinity : bounds[zone - 2];
    const from = offset(upper);
    const to = offset(lower);
    if (to > from) bands.push({ zone, from, to });
  }
  return bands;
}
