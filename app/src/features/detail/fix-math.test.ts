import type { LatLngTuple } from "leaflet";
import { describe, expect, it } from "vitest";

import type { FixSample } from "@/bindings/FixSample";

import {
  cumulativeMetres,
  editedDistance,
  editPath,
  insertVertex,
  joinRouted,
  metres,
  nearestSample,
  pathLength,
  stretchAround,
  toEdit,
} from "./fix-math";

/** One sample a second heading north, about 3 m apart; `gaps` have no position. */
function samples(count: number, gaps: number[] = []): FixSample[] {
  return Array.from({ length: count }, (_, i) => ({
    time: 1_746_522_000_000 + i * 1000,
    lat: gaps.includes(i) ? null : 48 + i * 0.000027,
    lon: gaps.includes(i) ? null : 2,
    elevation: null,
    heartRate: null,
    steps: null,
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

  it("finds the stretch around a dragged sample", () => {
    const track = samples(101, [30]);
    const cumulative = cumulativeMetres(track);
    expect(cumulative[30]).toBe(cumulative[29]);
    // About 3 m between samples: 30 m either way is 10 samples.
    const [start, end] = stretchAround(track, cumulative, 50, 30);
    expect(start).toBeGreaterThanOrEqual(39);
    expect(start).toBeLessThanOrEqual(41);
    expect(end).toBeGreaterThanOrEqual(59);
    expect(end).toBeLessThanOrEqual(61);
    expect(stretchAround(track, cumulative, 2, 30)[0]).toBe(0);
    expect(stretchAround(track, cumulative, 98, 30)[1]).toBe(100);
    // Never ends on a sample without a position.
    expect(stretchAround(track, cumulative, 35, 15)[0]).not.toBe(30);
  });

  it("inserts a point where it lengthens the route least", () => {
    const a: LatLngTuple = [48, 2];
    const b: LatLngTuple = [48.01, 2];
    const middle: LatLngTuple = [48.005, 2.001];
    const nearA: LatLngTuple = [48.001, 2.0005];
    expect(insertVertex(a, [middle], b, nearA)).toEqual([nearA, middle]);
    expect(insertVertex(null, [middle], null, nearA)).toEqual([middle, nearA]);
  });

  it("joins the routed line to the waypoints", () => {
    const a: LatLngTuple = [48, 2];
    const b: LatLngTuple = [48.01, 2];
    expect(
      joinRouted(
        [a, b],
        [
          [48.0001, 2],
          [48.0099, 2],
        ],
      ),
    ).toEqual([a, [48.0001, 2], [48.0099, 2], b]);
    expect(joinRouted([a, b], [])).toEqual([a, b]);
  });
});
