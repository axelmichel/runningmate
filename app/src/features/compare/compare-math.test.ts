import { describe, expect, it } from "vitest";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import type { RankedEffort } from "@/bindings/RankedEffort";
import type { SegmentRow } from "@/bindings/SegmentRow";
import type { TrackPoint } from "@/bindings/TrackPoint";

import {
  addActivities,
  commonDistance,
  compareColor,
  EMPTY_SELECTION,
  figuresFromDetail,
  figuresFromPoints,
  MAX_COMPARE,
  overlayRows,
  parseSelection,
  pickPasses,
  raceGaps,
  removeActivity,
  segmentSelection,
  sliceStretch,
  speedMode,
  splitRows,
  splitsComparable,
  timeAtKm,
  visibleFigures,
  type Figures,
} from "./compare-math";

/** A steady track: `pace` min/km, one point every 100 m up to `km`. */
function track(km: number, pace: number, extra: Partial<TrackPoint> = {}): TrackPoint[] {
  return Array.from({ length: Math.round(km * 10) + 1 }, (_, i) => ({
    distance: i / 10,
    elapsed: (i / 10) * pace * 60,
    latitude: 0,
    longitude: 0,
    elevation: 100 + (i % 2) * 2,
    heartRate: 150,
    pace,
    gap: pace,
    cadence: 170,
    ...extra,
  }));
}

const effort = (activityId: number, elapsed: number, date: number): RankedEffort => ({
  id: activityId * 10 + date,
  activityId,
  activityTitle: null,
  date,
  elapsed,
  avgHeartRate: null,
  startKm: 1,
  endKm: 2,
  rank: 0,
});

const noFigures: Figures = {
  distance: null,
  duration: null,
  pace: null,
  speed: null,
  gap: null,
  heartRate: null,
  elevationGain: null,
  calories: null,
  cadence: null,
};

describe("selection", () => {
  it("gives each activity the lowest free colour slot and keeps it", () => {
    let selection = addActivities(EMPTY_SELECTION, [5, 6, 7, 6]);
    expect(selection.items).toEqual([
      { id: 5, slot: 0 },
      { id: 6, slot: 1 },
      { id: 7, slot: 2 },
    ]);
    selection = removeActivity(selection, 6);
    expect(selection.items).toEqual([
      { id: 5, slot: 0 },
      { id: 7, slot: 2 },
    ]);
    selection = addActivities(selection, [8, 9]);
    expect(selection.items.map((i) => [i.id, i.slot])).toEqual([
      [5, 0],
      [7, 2],
      [8, 1],
      [9, 3],
    ]);
    expect(compareColor(3)).toBe("var(--compare-4)");
  });

  it("stops at ten activities", () => {
    const ids = Array.from({ length: 15 }, (_, i) => i + 1);
    expect(addActivities(EMPTY_SELECTION, ids).items).toHaveLength(MAX_COMPARE);
  });

  it("keeps the segment only while every activity has a pass", () => {
    const selection = segmentSelection({ id: 3, title: "Hill" }, [
      effort(1, 100, 1),
      effort(2, 90, 2),
    ]);
    expect(selection.segment?.stretches[2]).toEqual({ startKm: 1, endKm: 2 });
    expect(removeActivity(selection, 1).segment).not.toBeNull();
    expect(addActivities(selection, [99]).segment).toBeNull();
    expect(removeActivity(removeActivity(selection, 1), 2).segment).toBeNull();
  });

  it("restores a stored selection and drops what doesn't fit", () => {
    const stored = JSON.stringify({
      items: [
        { id: 1, slot: 0 },
        { id: 1, slot: 1 },
        { id: 2, slot: 0 },
        { id: 3, slot: 12 },
        { id: 4, slot: 4 },
      ],
      segment: { id: 9, title: "Hill", stretches: { 1: { startKm: 0, endKm: 1 } } },
    });
    expect(parseSelection(stored)).toEqual({
      items: [
        { id: 1, slot: 0 },
        { id: 4, slot: 4 },
      ],
      // Activity 4 has no pass, so the segment doesn't hold.
      segment: null,
    });
    expect(parseSelection("{oops")).toEqual(EMPTY_SELECTION);
    expect(parseSelection(null)).toEqual(EMPTY_SELECTION);
  });

  it("picks one pass per activity, fastest or latest, with the chosen one first", () => {
    const board = [effort(1, 100, 10), effort(2, 90, 20), effort(1, 80, 30), effort(3, 95, 40)];
    expect(pickPasses(board, "fastest").map((e) => [e.activityId, e.elapsed])).toEqual([
      [1, 80],
      [2, 90],
      [3, 95],
    ]);
    expect(pickPasses(board, "latest").map((e) => e.activityId)).toEqual([3, 1, 2]);
    expect(pickPasses(board, "fastest", 3).map((e) => e.activityId)).toEqual([3, 1, 2]);
  });
});

describe("sports", () => {
  it("shows pace for runs and walks, speed once a ride is in", () => {
    expect(speedMode(["Running", "Walking"])).toBe("pace");
    expect(speedMode(["Cycling"])).toBe("speed");
    expect(speedMode(["Running", "Cycling"])).toBe("speed");
  });

  it("compares splits only within per-km or per-5-km sports", () => {
    expect(splitsComparable(["Running", "Walking"])).toBe(true);
    expect(splitsComparable(["Cycling", "Cycling"])).toBe(true);
    expect(splitsComparable(["Running", "Cycling"])).toBe(false);
  });

  it("lines up the splits by number, as pace or speed", () => {
    const split = (pace: number | null, speed: number | null) => ({ pace, speed }) as SegmentRow;
    const activities = [
      { key: "a", splits: [split(5, 3), split(5.5, 3)] },
      { key: "b", splits: [split(null, null), split(6, 2.5), split(6.5, 2)] },
    ];
    expect(splitRows(activities, "pace")).toEqual([
      { split: 1, a: 5 },
      { split: 2, a: 5.5, b: 6 },
      { split: 3, b: 6.5 },
    ]);
    expect(splitRows(activities, "speed")[1]).toEqual({ split: 2, a: 10.8, b: 9 });
  });

  it("hides figures nobody has and GAP and steps of rides", () => {
    const run = { view: "Running" as const, figures: { ...noFigures, distance: 10, pace: 5 } };
    const ride = {
      view: "Cycling" as const,
      figures: { ...noFigures, distance: 40, speed: 30, gap: 4, cadence: 80 },
    };
    expect(visibleFigures([run])).toEqual(["distance", "pace"]);
    expect(visibleFigures([ride])).toEqual(["distance", "speed"]);
    expect(visibleFigures([run, ride])).toEqual(["distance", "speed"]);
  });

  it("reads the stored figures and fills in pace and speed", () => {
    const detail = {
      distance: 10,
      duration: 3000,
      avgPace: "05:00",
      avgSpeed: null,
      avgGap: null,
      avgHeartRate: 0,
      elevationGain: 0,
      calories: 600,
      avgSteps: 172,
    } as ActivityDetail;
    expect(figuresFromDetail(detail)).toEqual({
      distance: 10,
      duration: 3000,
      pace: 5,
      speed: 12,
      gap: null,
      heartRate: null,
      elevationGain: 0,
      calories: 600,
      cadence: 172,
    });
  });
});

describe("series", () => {
  it("interpolates the time at a distance", () => {
    const points = track(1, 5);
    expect(timeAtKm(points, 0.25)).toBeCloseTo(75);
    expect(timeAtKm(points, 1)).toBeCloseTo(300);
    expect(timeAtKm(points, 1.01)).toBeNull();
    expect(timeAtKm([], 0)).toBeNull();
  });

  it("finds the distance every activity covers", () => {
    expect(commonDistance([track(5, 5), track(3.2, 6), track(4, 4)])).toBeCloseTo(3.2);
    expect(commonDistance([])).toBe(0);
  });

  it("measures the race against the fastest up to the common distance", () => {
    const race = raceGaps(
      [
        { key: "slow", points: track(5, 6) },
        { key: "fast", points: track(4, 5) },
      ],
      4,
    );
    expect(race?.leader).toBe("fast");
    expect(race?.distance).toBeCloseTo(4);
    expect(race?.rows.map((r) => r.distance)).toEqual([0, 1, 2, 3, 4]);
    // One minute per km behind.
    expect(race?.rows.map((r) => Math.round(r.slow))).toEqual([0, 60, 120, 180, 240]);
    expect(race?.rows.every((r) => r.fast === 0)).toBe(true);
    expect(raceGaps([{ key: "alone", points: track(4, 5) }])).toBeNull();
  });

  it("cuts a stretch out and counts from its start", () => {
    const stretch = sliceStretch(track(3, 5), { startKm: 1, endKm: 2 });
    expect(stretch[0].distance).toBeCloseTo(0);
    expect(stretch[0].elapsed).toBeCloseTo(0);
    expect(stretch.at(-1)?.distance).toBeCloseTo(1);
    expect(stretch.at(-1)?.elapsed).toBeCloseTo(300);
    const figures = figuresFromPoints(stretch);
    expect(figures.pace).toBeCloseTo(5);
    expect(figures.speed).toBeCloseTo(12);
    expect(figures.heartRate).toBe(150);
    expect(figures.calories).toBeNull();
  });

  it("averages each activity per bin for the overlay", () => {
    const rows = overlayRows(
      [
        { key: "a", points: track(1, 5) },
        { key: "b", points: track(2, 6, { heartRate: null }) },
      ],
      (p) => p.heartRate,
      "distance",
      4,
    );
    // Only "a" has a heart rate, up to its end at 1 km.
    expect(rows.map((r) => r.x)).toEqual([0.25, 0.75, 1.25]);
    expect(rows.every((r) => r.a === 150 && !("b" in r))).toBe(true);
  });
});
