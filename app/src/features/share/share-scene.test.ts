import { describe, expect, it } from "vitest";

import type { ActivityDetail } from "@/bindings/ActivityDetail";

import {
  DEFAULT_SCENE,
  gridLines,
  infoItems,
  mercator,
  routeOf,
  routeTransform,
  shareFileName,
  snapToGrid,
  tilesFor,
  type Scene,
} from "./share-scene";

const scene = (patch: Partial<Scene["route"]> = {}): Scene => ({
  ...DEFAULT_SCENE,
  width: 1000,
  height: 500,
  route: { ...DEFAULT_SCENE.route, x: 0.5, y: 0.5, size: 0.8, ...patch },
});

const run = {
  id: 1,
  viewMode: "Running",
  date: 1_700_000_000,
  title: "Morning run",
  distance: 10.5,
  duration: 3125,
  avgPace: "04:58",
  avgSpeed: 12.1,
  elevationGain: 85,
  avgHeartRate: 151,
  calories: null,
} as ActivityDetail;

describe("mercator", () => {
  it("puts null island in the middle and north up", () => {
    expect(mercator(0, 0)).toEqual([0.5, 0.5]);
    const [, north] = mercator(48, 11);
    expect(north).toBeLessThan(0.5);
  });
});

describe("routeTransform", () => {
  it("fits the longer side to the size and centres the route on its placement", () => {
    const route = routeOf([
      { latitude: 48.0, longitude: 11.0 },
      { latitude: 48.01, longitude: 11.04 },
    ])!;
    const s = scene({ x: 0.25, size: 0.8 });
    const { scale, offsetX, offsetY } = routeTransform(route, s);
    const [x0, y0, x1, y1] = route.box;
    expect((x1 - x0) * scale).toBeCloseTo(400);
    expect(((x0 + x1) / 2) * scale + offsetX).toBeCloseTo(250);
    expect(((y0 + y1) / 2) * scale + offsetY).toBeCloseTo(250);
  });

  it("has no route without two points", () => {
    expect(routeOf([{ latitude: 48, longitude: 11 }])).toBeNull();
  });
});

describe("tilesFor", () => {
  it("covers the canvas with tiles of about 256 px", () => {
    const transform = {
      scale: 256 * 2 ** 10,
      offsetX: -0.5 * 256 * 2 ** 10,
      offsetY: -0.5 * 256 * 2 ** 10,
    };
    const tiles = tilesFor(transform, 600, 300);
    expect(new Set(tiles.map((t) => t.z))).toEqual(new Set([10]));
    expect(tiles[0].side).toBeCloseTo(256);
    const right = Math.max(...tiles.map((t) => t.left + t.side));
    const bottom = Math.max(...tiles.map((t) => t.top + t.side));
    expect(Math.min(...tiles.map((t) => t.left))).toBeLessThanOrEqual(0);
    expect(right).toBeGreaterThanOrEqual(600);
    expect(bottom).toBeGreaterThanOrEqual(300);
    expect(tiles).toHaveLength(3 * 2);
  });

  it("never asks for zooms the server doesn't have", () => {
    const tiles = tilesFor({ scale: 256 * 2 ** 20, offsetX: 0, offsetY: 0 }, 100, 100);
    expect(tiles.every((t) => t.z === 16)).toBe(true);
  });
});

describe("infoItems", () => {
  it("gives each chosen fact on its own and skips missing ones", () => {
    const items = infoItems(run, ["title", "distance", "pace", "calories"]);
    expect(items.map((i) => i.key)).toEqual(["title", "distance", "pace"]);
    expect(items[0]).toEqual({ key: "title", value: "Morning run", label: null });
    expect(items[1].label).toBe("Distance");
    expect(items[1].value).toMatch(/^10[.,]50 km$/);
  });

  it("shows speed instead of pace for rides", () => {
    const [pace] = infoItems({ ...run, viewMode: "Cycling" }, ["pace"]);
    expect(pace.label).toBe("Average speed");
    expect(pace.value).toMatch(/km\/h$/);
  });
});

describe("shareFileName", () => {
  it("uses the title and the day", () => {
    expect(shareFileName(run)).toMatch(/^Morning run \d{4}-\d{2}-\d{2}$/);
  });
});

describe("grid", () => {
  it("runs a line through the centre and steps out to both edges", () => {
    expect(gridLines(1000, 300)).toEqual([200, 500, 800]);
    expect(gridLines(1000, 250)).toEqual([0, 250, 500, 750, 1000]);
  });

  it("snaps to the nearest line", () => {
    expect(snapToGrid(480, 1000, 300)).toBe(500);
    expect(snapToGrid(700, 1000, 300)).toBe(800);
    expect(snapToGrid(510, 1000, 100)).toBe(500);
  });
});
