import { describe, expect, it } from "vitest";

import type { SegmentRow } from "@/bindings/SegmentRow";
import type { TrackPoint } from "@/bindings/TrackPoint";

import { createHoverStore, indexAtKm, speedFromPace, splitEnds } from "./track-position";

const row = (index: number, distance: number | null): SegmentRow => ({
  index,
  distance,
  duration: null,
  pace: null,
  gap: null,
  speed: null,
  heartRate: null,
  power: null,
  steps: null,
  elevationGain: null,
  start: null,
  end: null,
});

const points = [0, 0.4, 1, 1.6, 2.05].map((distance): TrackPoint => ({
  distance,
  elapsed: 0,
  latitude: 0,
  longitude: 0,
  elevation: 0,
  heartRate: null,
  pace: null,
  gap: null,
  cadence: null,
}));

describe("track position", () => {
  it("adds the splits up", () => {
    expect(splitEnds([row(0, 1), row(1, 1), row(2, 0.05)])).toEqual([1, 2, 2.05]);
    expect(splitEnds([row(0, null), row(1, 1)])).toEqual([0, 1]);
  });

  it("finds the point at a distance", () => {
    expect(indexAtKm(points, 1)).toBe(2);
    expect(indexAtKm(points, 1.1)).toBe(3);
    expect(indexAtKm(points, 9)).toBe(4);
  });

  it("turns pace into speed", () => {
    expect(speedFromPace(5)).toBe(12);
    expect(speedFromPace(null)).toBeNull();
  });

  it("tells subscribers about a new hover only", () => {
    const store = createHoverStore();
    let calls = 0;
    const stop = store.subscribe(() => calls++);
    store.set(3);
    store.set(3);
    store.set(null);
    stop();
    store.set(1);
    expect(calls).toBe(2);
    expect(store.get()).toBe(1);
  });
});
