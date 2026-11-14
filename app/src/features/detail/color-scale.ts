/** Colour ramps for the pace and heart-rate maps (after the folium/branca maps in Python). */

type Rgb = [number, number, number];

const hex = (value: string): Rgb => [
  parseInt(value.slice(1, 3), 16),
  parseInt(value.slice(3, 5), 16),
  parseInt(value.slice(5, 7), 16),
];

/** Fast → slow: yellow to purple (reversed "plasma", as in `track_map.py`). */
export const PACE_RAMP = ["#f0f921", "#f89540", "#cc4778", "#7e03a8", "#0d0887"].map(hex);
/** Low → high heart rate: blue to red (reversed "Spectral"). */
export const HEART_RATE_RAMP = ["#2b83ba", "#abdda4", "#fdae61", "#d7191c"].map(hex);

/** Colour at `t` (0–1, clamped) along a ramp, as `rgb(r g b)`. */
export function rampColor(ramp: Rgb[], t: number): string {
  const clamped = Math.min(1, Math.max(0, Number.isFinite(t) ? t : 0));
  const position = clamped * (ramp.length - 1);
  const index = Math.min(ramp.length - 2, Math.floor(position));
  const f = position - index;
  const [a, b] = [ramp[index], ramp[index + 1]];
  const mix = (i: number) => Math.round(a[i] + (b[i] - a[i]) * f);
  return `rgb(${mix(0)} ${mix(1)} ${mix(2)})`;
}

/** 5th–95th percentile range, so single outliers don't flatten the colours. */
export function robustRange(values: number[]): [number, number] {
  const sorted = values.filter(Number.isFinite).sort((x, y) => x - y);
  if (sorted.length === 0) return [0, 1];
  const at = (q: number) => sorted[Math.round(q * (sorted.length - 1))];
  const [low, high] = [at(0.05), at(0.95)];
  return low === high ? [low - 1, high + 1] : [low, high];
}
