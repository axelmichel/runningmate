import type { LatLngTuple } from "leaflet";
import { describe, expect, it } from "vitest";

import type { FixSample } from "@/bindings/FixSample";

import { editedDistance, editPath, metres, nearestSample, pathLength, toEdit } from "./fix-math";

/** One sample a second heading north, about 3 m apart; `gaps` have no position. */
function samples(count: number, gaps: number[] = []): FixSample[] {
  return Array.from({ length: count }, (_, i) => ({
    time: 1_746_522_000_000 + i * 1000,
    lat: gaps.includes(i) ? null : 48 + i * 0.000027,
    lon: gaps.includes(i) ? null : 2,
    elevation: null,
    heartRate: null,
  }));
}

describe("fix math", () => {
  it("measures great-circle distances", () => {
    // One degree of latitude is about 111.2 km.
    expect(metres([48, 2], [49, 2])).toBeCloseTo(111_195, -2);
    expect(pathLength([[48, 2]])).toBe(0);
  });

  it("puts the anchors around the drawn points", () => {
    const track = samples(10, [0, 5]);
    const drawn: LatLngTuple[] = [[48.1, 2.1]];
    expect(editPath(track, 0, 9, drawn)).toEqual([
      [48.1, 2.1],
      [48 + 9 * 0.000027, 2],
    ]);
    expect(editPath(track, 4, 6, drawn)).toHaveLength(3);
  });

  it("previews the distance with a stretch redrawn", () => {
    const track = samples(101);
    const recorded = editedDistance(track, []);
    expect(recorded).toBeCloseTo(300, -1);
    // Samples 40–60 redrawn as a detour 100 m east and back.
    const a: LatLngTuple = [track[40].lat!, 2];
    const b: LatLngTuple = [track[60].lat!, 2];
    const east: LatLngTuple = [a[0], 2 + 100 / 73_700];
    const edit = toEdit(track, 40, 60, [a, east, b]);
    expect(editedDistance(track, [edit])).toBeCloseTo(
      recorded - metres(a, b) + pathLength([a, east, b]),
      3,
    );
  });

  it("finds the nearest positioned sample within the limit", () => {
    const track = samples(10, [3]);
    const target: LatLngTuple = [48 + 3 * 0.000027, 2];
    const near = (p: LatLngTuple) => metres(p, target);
    expect([2, 4]).toContain(nearestSample(track, near, 10));
    expect(nearestSample(track, near, 1)).toBeNull();
  });
});
