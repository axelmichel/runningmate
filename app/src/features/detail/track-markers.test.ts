import { describe, expect, it } from "vitest";

import type { TrackPoint } from "@/bindings/TrackPoint";

import { bearing, directionArrows, snapToPass } from "./track-markers";

const point = (distance: number, latitude: number, longitude: number): TrackPoint => ({
  distance,
  elapsed: 0,
  latitude,
  longitude,
  elevation: 0,
  heartRate: null,
  pace: null,
  cadence: null,
});

/** 2 km due north, a point every 100 m. */
const north = Array.from({ length: 21 }, (_, i) => point(i / 10, 48 + (i * 100) / 111_195, 11));

describe("track markers", () => {
  it("measures the bearing on the map", () => {
    expect(bearing(north[0], north[1])).toBeCloseTo(0);
    expect(bearing(point(0, 48, 11), point(0.1, 48, 11.01))).toBeCloseTo(90);
    expect(bearing(north[1], north[0])).toBeCloseTo(180);
    expect(bearing(point(0, 48, 11.01), point(0.1, 48, 11))).toBeCloseTo(270);
  });

  it("points arrows the way the track was recorded", () => {
    const arrows = directionArrows(north);
    // 2 km / 40 → every 100 m, starting halfway.
    expect(arrows).toHaveLength(20);
    expect(arrows.every((a) => Math.abs(a.bearing) < 1e-6)).toBe(true);
    expect(arrows[0].position[0]).toBeCloseTo(48 + 50 / 111_195);
    const back = directionArrows([...north].reverse().map((p, i) => ({ ...p, distance: i / 10 })));
    expect(back.every((a) => Math.abs(a.bearing - 180) < 1e-6)).toBe(true);
  });

  it("limits arrows to a stretch", () => {
    const arrows = directionArrows(north, 0.5, 1);
    expect(arrows.length).toBeGreaterThan(0);
    expect(arrows.every((a) => a.position[0] > north[5].latitude)).toBe(true);
    expect(arrows.every((a) => a.position[0] < north[10].latitude)).toBe(true);
  });

  it("snaps to the first pass, or the next one after a start", () => {
    // Out and back past the same spot: near at 2–4 (out) and 14–16 (back).
    const pixels = [50, 30, 10, 3, 12, 40, 80, 120, 200, 120, 80, 40, 30, 25, 9, 1, 8, 30, 60];
    expect(snapToPass(pixels, 20)).toBe(3);
    // The end after a start at index 6 is the way back, not the closer-looking outbound point.
    expect(snapToPass(pixels, 20, 6)).toBe(15);
    expect(snapToPass(pixels, 0.5)).toBeNull();
  });
});
