import { describe, expect, it } from "vitest";

import type { FixSample } from "@/bindings/FixSample";
import type { SegmentRow } from "@/bindings/SegmentRow";

import {
  detectHeartRateIssues,
  detectStepsIssues,
  previewSampleEdits,
  splitEdits,
} from "./sample-fix-math";

const T0 = 1_746_522_000_000;

/** One sample a second heading north at about 3 m/s. */
function samples(heartRates: (number | null)[], steps?: (number | null)[]): FixSample[] {
  return heartRates.map((heartRate, i) => ({
    time: T0 + i * 1000,
    lat: 48 + i * 0.000027,
    lon: 2,
    elevation: 10,
    heartRate,
    steps: steps ? steps[i] : 170,
  }));
}

const at = (second: number) => T0 + second * 1000;

describe("previewSampleEdits", () => {
  it("interpolates, sets and shifts like the backend", () => {
    const track = samples([100, 190, 0, 185, 140]);
    const interpolated = previewSampleEdits(track, [
      { channel: "heartRate", from: at(1), to: at(3), op: { kind: "interpolate" } },
    ]);
    expect(interpolated.map((s) => s.heartRate)).toEqual([100, 110, 120, 130, 140]);

    const atStart = previewSampleEdits(samples([175, 178, 0, 142]), [
      { channel: "heartRate", from: at(0), to: at(2), op: { kind: "interpolate" } },
    ]);
    expect(atStart.map((s) => s.heartRate)).toEqual([142, 142, 142, 142]);

    const steps = previewSampleEdits(samples([140, 140, 140]), [
      { channel: "steps", from: at(0), to: at(1), op: { kind: "set", value: 160 } },
      { channel: "steps", from: at(1), to: at(2), op: { kind: "offset", value: -10 } },
    ]);
    expect(steps.map((s) => s.steps)).toEqual([160, 150, 160]);
  });
});

describe("detectHeartRateIssues", () => {
  it("finds nothing on a steady run", () => {
    expect(detectHeartRateIssues(samples(Array(120).fill(145)))).toEqual([]);
  });

  it("finds a spike that comes back", () => {
    const hr = Array(60).fill(145);
    for (let i = 20; i < 25; i++) hr[i] = 200;
    expect(detectHeartRateIssues(samples(hr))).toEqual([{ kind: "spike", start: 20, end: 24 }]);
  });

  it("finds a strap settling in at the start", () => {
    const hr = [...Array(120).fill(185), ...Array(300).fill(140)];
    expect(detectHeartRateIssues(samples(hr))).toEqual([
      { kind: "startArtifact", start: 0, end: 119 },
    ]);
  });

  it("finds heart rate locked onto the steps", () => {
    const hr = [...Array(60).fill(168), ...Array(60).fill(145)];
    const steps = Array(120).fill(170);
    const issues = detectHeartRateIssues(samples(hr, steps));
    expect(issues.map((i) => i.kind)).toContain("cadenceLock");
    expect(issues[0].start).toBe(0);
  });

  it("finds a dropout", () => {
    const hr: (number | null)[] = Array(60).fill(145);
    for (let i = 20; i < 35; i++) hr[i] = null;
    expect(detectHeartRateIssues(samples(hr))).toEqual([{ kind: "dropout", start: 20, end: 34 }]);
  });
});

describe("detectStepsIssues", () => {
  it("finds steps missing while moving and impossible rates", () => {
    const steps: (number | null)[] = Array(60).fill(170);
    for (let i = 10; i < 25; i++) steps[i] = null;
    steps[40] = 400;
    expect(detectStepsIssues(samples(Array(60).fill(140), steps))).toEqual([
      { kind: "dropout", start: 10, end: 24 },
      { kind: "spike", start: 40, end: 40 },
    ]);
  });
});

describe("splitEdits", () => {
  const row = (index: number, start: number, end: number): SegmentRow => ({
    index,
    distance: 1,
    duration: 300,
    pace: 5,
    gap: 5,
    speed: 3.3,
    heartRate: 150,
    power: null,
    steps: 85,
    elevationGain: 0,
    start: at(start),
    end: at(end),
  });

  it("shifts a split's samples to the entered averages", () => {
    const rows = [row(0, 0, 300), row(1, 300, 600)];
    expect(splitEdits(rows, { 0: { heartRate: 140, steps: 170 }, 1: { steps: 176 } })).toEqual([
      { channel: "heartRate", from: at(0), to: at(300) - 1, op: { kind: "offset", value: -10 } },
      { channel: "steps", from: at(300), to: at(600), op: { kind: "offset", value: 6 } },
    ]);
  });
});
