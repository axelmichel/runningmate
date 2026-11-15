import { describe, expect, it } from "vitest";

import type { ActivitySegmentEffort } from "@/bindings/ActivitySegmentEffort";
import type { TrackPoint } from "@/bindings/TrackPoint";

import { gradeSeries, selectionStats, trophyCount } from "./segment-math";

/** A point every 100 m due north, climbing 5 m each. */
const points: TrackPoint[] = Array.from({ length: 11 }, (_, i) => ({
  distance: i / 10,
  elapsed: i * 30,
  latitude: 48 + (i * 100) / 111_195,
  longitude: 11,
  elevation: 100 + i * 5,
  heartRate: null,
  pace: null,
  cadence: null,
}));

function effort(segmentId: number, rank: number): ActivitySegmentEffort {
  return {
    id: segmentId * 10 + rank,
    segmentId,
    title: "",
    distance: 500,
    avgGrade: 0,
    elevationGain: 0,
    startKm: 0,
    endKm: 0.5,
    elapsed: 100,
    avgHeartRate: null,
    rank,
    tries: 5,
  };
}

describe("segment math", () => {
  it("measures a selection", () => {
    const stats = selectionStats(points, 0.2, 0.6);
    expect(stats.distance).toBeCloseTo(0.4);
    expect(stats.elevationGain).toBeCloseTo(20);
    expect(stats.avgGrade).toBeCloseTo(5);
  });

  it("measures the grade along a profile", () => {
    const profile = Array.from({ length: 21 }, (_, i) => ({
      distance: i * 10,
      elevation: i < 10 ? 100 : 100 + (i - 10) * 1,
    }));
    const grades = gradeSeries(profile);
    expect(grades[2]).toBeCloseTo(0);
    expect(grades[18]).toBeCloseTo(10);
  });

  it("counts each segment in the top three once", () => {
    expect(trophyCount([effort(1, 1), effort(1, 2), effort(2, 3), effort(3, 4)])).toBe(2);
    expect(trophyCount([])).toBe(0);
  });
});
