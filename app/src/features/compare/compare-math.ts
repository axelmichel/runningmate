import type { ActivityDetail } from "@/bindings/ActivityDetail";
import type { RankedEffort } from "@/bindings/RankedEffort";
import type { SegmentRow } from "@/bindings/SegmentRow";
import type { TrackPoint } from "@/bindings/TrackPoint";
import type { ViewMode } from "@/bindings/ViewMode";

/** Activities compared at once, one colour each. */
export const MAX_COMPARE = 10;

/** Colour of a slot (0-based), the `--compare-*` variables in index.css. */
export function compareColor(slot: number): string {
  return `var(--compare-${(slot % MAX_COMPARE) + 1})`;
}

/** A compared activity; the slot picks its colour and stays while others come and go. */
export type CompareItem = { id: number; slot: number };

/** Where a segment pass lies within its activity, km. */
export type Stretch = { startKm: number; endKm: number };

/** Set while the selection is the passes of one segment. */
export type CompareSegment = {
  id: number;
  title: string;
  /** The pass of each activity, by activity id. */
  stretches: Record<number, Stretch>;
};

export type CompareSelection = { items: CompareItem[]; segment: CompareSegment | null };

export const EMPTY_SELECTION: CompareSelection = { items: [], segment: null };

/**
 * Adds activities (skipping those already in) into the lowest free slots, up to
 * [`MAX_COMPARE`]. The segment is dropped once an activity without a pass over it joins.
 */
export function addActivities(selection: CompareSelection, ids: number[]): CompareSelection {
  const items = [...selection.items];
  let segment = selection.segment;
  for (const id of ids) {
    if (items.length >= MAX_COMPARE) break;
    if (items.some((item) => item.id === id)) continue;
    const used = new Set(items.map((item) => item.slot));
    let slot = 0;
    while (used.has(slot)) slot++;
    items.push({ id, slot });
    if (segment && !segment.stretches[id]) segment = null;
  }
  return { items, segment };
}

export function removeActivity(selection: CompareSelection, id: number): CompareSelection {
  const items = selection.items.filter((item) => item.id !== id);
  return { items, segment: items.length ? selection.segment : null };
}

/**
 * One pass per activity, up to [`MAX_COMPARE`]: each activity's fastest pass, fastest first, or
 * its latest one, newest first. `first` (an activity) leads when it has a pass.
 */
export function pickPasses(
  leaderboard: RankedEffort[],
  order: "fastest" | "latest",
  first?: number,
): RankedEffort[] {
  const sorted = [...leaderboard].sort((a, b) =>
    order === "fastest" ? a.elapsed - b.elapsed || a.date - b.date : b.date - a.date,
  );
  const picked: RankedEffort[] = [];
  const lead = sorted.find((effort) => effort.activityId === first);
  if (lead) picked.push(lead);
  for (const effort of sorted) {
    if (picked.length >= MAX_COMPARE) break;
    if (!picked.some((p) => p.activityId === effort.activityId)) picked.push(effort);
  }
  return picked;
}

/** A new selection of segment passes, each activity limited to its pass. */
export function segmentSelection(
  segment: { id: number; title: string },
  passes: RankedEffort[],
): CompareSelection {
  const stretches: Record<number, Stretch> = {};
  for (const pass of passes) {
    stretches[pass.activityId] = { startKm: pass.startKm, endKm: pass.endKm };
  }
  return addActivities(
    { items: [], segment: { id: segment.id, title: segment.title, stretches } },
    passes.map((pass) => pass.activityId),
  );
}

/** The stored selection; anything unreadable gives an empty one. */
export function parseSelection(json: string | null): CompareSelection {
  try {
    const value = JSON.parse(json ?? "null") as Partial<CompareSelection> | null;
    const items = (Array.isArray(value?.items) ? value.items : []).filter(
      (item): item is CompareItem =>
        Number.isInteger(item?.id) &&
        Number.isInteger(item?.slot) &&
        item.slot >= 0 &&
        item.slot < MAX_COMPARE,
    );
    const unique = items
      .filter(
        (item, i) =>
          items.findIndex((other) => other.id === item.id || other.slot === item.slot) === i,
      )
      .slice(0, MAX_COMPARE);
    const segment = value?.segment;
    const valid =
      segment != null &&
      Number.isInteger(segment.id) &&
      typeof segment.title === "string" &&
      typeof segment.stretches === "object" &&
      unique.every((item) => segment.stretches?.[item.id]);
    return { items: unique, segment: valid && unique.length ? segment : null };
  } catch {
    return EMPTY_SELECTION;
  }
}

/**
 * How speed is shown: pace for runs and walks, speed once a ride is among them. With mixed
 * sports everything is shown as speed, so all activities share one axis.
 */
export type SpeedMode = "pace" | "speed";

export function speedMode(views: ViewMode[]): SpeedMode {
  return views.includes("Cycling") ? "speed" : "pace";
}

/** Runs and walks have steps and a grade-adjusted pace; rides don't. */
export function onFoot(view: ViewMode): boolean {
  return view !== "Cycling";
}

/** Whether splits line up: rides are split per 5 km, runs and walks per km. */
export function splitsComparable(views: ViewMode[]): boolean {
  return views.every(onFoot) || views.every((view) => !onFoot(view));
}

/**
 * One row per split number with each activity's pace (min/km) or speed (km/h) in it, for
 * the splits side by side.
 */
export function splitRows(
  activities: { key: string; splits?: SegmentRow[] }[],
  mode: SpeedMode,
): ({ split: number } & Record<string, number>)[] {
  const rows: ({ split: number } & Record<string, number>)[] = [];
  for (const { key, splits = [] } of activities) {
    splits.forEach((split, i) => {
      const value = mode === "pace" ? split.pace : split.speed != null ? split.speed * 3.6 : null;
      if (!value || !Number.isFinite(value)) return;
      rows[i] ??= { split: i + 1 };
      rows[i][key] = value;
    });
  }
  return rows.filter(Boolean);
}

/**
 * The points within `stretch`, with distance and time counted from its start (the whole
 * series without one).
 */
export function sliceStretch(points: TrackPoint[], stretch: Stretch | null): TrackPoint[] {
  if (!stretch) return points;
  const inside = points.filter(
    (p) => p.distance >= stretch.startKm - 1e-9 && p.distance <= stretch.endKm + 1e-9,
  );
  if (!inside.length) return [];
  const start = timeAtKm(points, stretch.startKm) ?? inside[0].elapsed;
  return inside.map((p) => ({
    ...p,
    distance: Math.max(0, p.distance - stretch.startKm),
    elapsed: Math.max(0, p.elapsed - start),
  }));
}

/** Seconds since the start at `km`, interpolated between points; `null` beyond the end. */
export function timeAtKm(points: TrackPoint[], km: number): number | null {
  if (!points.length || km < points[0].distance || km > points[points.length - 1].distance) {
    return null;
  }
  // The first point at or beyond `km` (distances never decrease).
  let lo = 0;
  let hi = points.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (points[mid].distance < km) lo = mid + 1;
    else hi = mid;
  }
  const after = points[lo];
  const before = points[lo - 1];
  if (!before || after.distance === before.distance) return after.elapsed;
  const share = (km - before.distance) / (after.distance - before.distance);
  return before.elapsed + share * (after.elapsed - before.elapsed);
}

/** The distance every series covers (km), 0 without any. */
export function commonDistance(series: TrackPoint[][]): number {
  if (!series.length) return 0;
  return Math.max(0, Math.min(...series.map((points) => points.at(-1)?.distance ?? 0)));
}

export type Series = { key: string; points: TrackPoint[] };

export type RaceRow = { distance: number } & Record<string, number>;

/**
 * The virtual race: seconds behind (positive) or ahead of the fastest activity over the common
 * distance, at `steps` + 1 evenly spaced distances. The fastest is the one first at the end of
 * the common distance. `null` with fewer than two series.
 */
export function raceGaps(
  series: Series[],
  steps = 200,
): { rows: RaceRow[]; leader: string; distance: number } | null {
  const usable = series.filter((s) => s.points.length > 1);
  if (usable.length < 2) return null;
  const distance = commonDistance(usable.map((s) => s.points));
  if (!(distance > 0)) return null;
  const finish = usable.map((s) => timeAtKm(s.points, distance) ?? Infinity);
  const leader = usable[finish.indexOf(Math.min(...finish))];
  const rows: RaceRow[] = [];
  for (let i = 0; i <= steps; i++) {
    const km = (distance * i) / steps;
    const lead = timeAtKm(leader.points, km);
    if (lead == null) continue;
    const row: RaceRow = { distance: km };
    for (const s of usable) {
      const time = timeAtKm(s.points, km);
      if (time != null) row[s.key] = time - lead;
    }
    rows.push(row);
  }
  return { rows, leader: leader.key, distance };
}

export type OverlayRow = { x: number } & Record<string, number>;

/**
 * One row per bin of the x axis (distance in km or time in s), with each series' average value
 * in that bin, for lines of different activities in one chart. Bins without any value are left
 * out; a series without a value in a bin has no key there.
 */
export function overlayRows(
  series: Series[],
  value: (point: TrackPoint) => number | null,
  axis: "distance" | "elapsed",
  bins = 200,
): OverlayRow[] {
  const end = Math.max(0, ...series.map((s) => s.points.at(-1)?.[axis] ?? 0));
  if (!(end > 0)) return [];
  const size = end / bins;
  const sums = new Map<number, Map<string, { sum: number; count: number }>>();
  for (const s of series) {
    for (const point of s.points) {
      const v = value(point);
      if (v == null || !Number.isFinite(v)) continue;
      const bin = Math.min(bins - 1, Math.floor(point[axis] / size));
      let row = sums.get(bin);
      if (!row) sums.set(bin, (row = new Map()));
      const cell = row.get(s.key) ?? { sum: 0, count: 0 };
      cell.sum += v;
      cell.count++;
      row.set(s.key, cell);
    }
  }
  return [...sums.entries()]
    .sort(([a], [b]) => a - b)
    .map(([bin, cells]) => {
      const row: OverlayRow = { x: (bin + 0.5) * size };
      for (const [key, { sum, count }] of cells) row[key] = sum / count;
      return row;
    });
}

/** The numbers compared in the key figures; `null` where an activity has none. */
export type Figures = {
  /** km. */
  distance: number | null;
  /** s. */
  duration: number | null;
  /** min/km. */
  pace: number | null;
  /** km/h. */
  speed: number | null;
  /** min/km. */
  gap: number | null;
  /** bpm. */
  heartRate: number | null;
  /** m. */
  elevationGain: number | null;
  /** kcal. */
  calories: number | null;
  /** Steps per minute. */
  cadence: number | null;
};

export type FigureKey = keyof Figures;

export const FIGURE_KEYS: FigureKey[] = [
  "distance",
  "duration",
  "pace",
  "speed",
  "gap",
  "heartRate",
  "elevationGain",
  "calories",
  "cadence",
];

/** "MM:SS" (as stored) → min/km. */
function paceMinutes(value: string | null): number | null {
  const match = value?.match(/^(\d+):(\d+)$/);
  const minutes = match ? Number(match[1]) + Number(match[2]) / 60 : 0;
  return minutes > 0 ? minutes : null;
}

const positive = (value: number | null | undefined) => (value && value > 0 ? value : null);

/** Pace and speed from distance and time, where the stored average is missing. */
function fillSpeed(figures: Figures): Figures {
  const { distance, duration } = figures;
  const moving = distance && duration ? { distance, duration } : null;
  return {
    ...figures,
    pace: figures.pace ?? (moving ? moving.duration / 60 / moving.distance : null),
    speed: figures.speed ?? (moving ? moving.distance / (moving.duration / 3600) : null),
  };
}

/** The whole activity's figures, as stored. */
export function figuresFromDetail(detail: ActivityDetail): Figures {
  return fillSpeed({
    distance: positive(detail.distance),
    duration: positive(detail.duration),
    pace: paceMinutes(detail.avgPace),
    speed: positive(detail.avgSpeed),
    gap: paceMinutes(detail.avgGap),
    heartRate: positive(detail.avgHeartRate),
    elevationGain: detail.elevationGain ?? null,
    calories: positive(detail.calories),
    cadence: positive(detail.avgSteps),
  });
}

const mean = (values: (number | null)[]) => {
  const present = values.filter((v): v is number => v != null);
  return present.length ? present.reduce((a, b) => a + b, 0) / present.length : null;
};

/** Figures of a stretch (from [`sliceStretch`]), measured on its points; no calories. */
export function figuresFromPoints(points: TrackPoint[]): Figures {
  let elevationGain = 0;
  for (let i = 1; i < points.length; i++) {
    elevationGain += Math.max(0, points[i].elevation - points[i - 1].elevation);
  }
  const last = points.at(-1);
  return fillSpeed({
    distance: positive(last?.distance),
    duration: positive(last?.elapsed),
    pace: null,
    speed: null,
    gap: mean(points.map((p) => p.gap)),
    heartRate: mean(points.map((p) => p.heartRate)),
    elevationGain: points.length > 1 ? elevationGain : null,
    calories: null,
    cadence: mean(points.map((p) => p.cadence)),
  });
}

/**
 * The figures worth a row: pace or speed by [`speedMode`], and only those some activity has.
 * GAP and steps of rides are dropped (rides have neither).
 */
export function visibleFigures(entries: { view: ViewMode; figures: Figures }[]): FigureKey[] {
  const mode = speedMode(entries.map((e) => e.view));
  return FIGURE_KEYS.filter((key) => {
    if (key === "pace" && mode !== "pace") return false;
    if (key === "speed" && mode !== "speed") return false;
    return entries.some((e) => figureValue(e, key) != null);
  });
}

/** One figure of an activity, `null` where it doesn't apply to the sport. */
export function figureValue(
  entry: { view: ViewMode; figures: Figures },
  key: FigureKey,
): number | null {
  if ((key === "gap" || key === "cadence") && !onFoot(entry.view)) return null;
  return entry.figures[key];
}
