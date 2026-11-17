import { describe, expect, it } from "vitest";

import type { FixSample } from "@/bindings/FixSample";
import type { SegmentRow } from "@/bindings/SegmentRow";

import {
  detectHeartRateIssues,
  detectStepsIssues,
  estimateStretch,
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
    expect(detectHeartRateIssues(samples(Array(300).fill(145)))).toEqual([]);
  });

  it("finds a strap that only connects after a while", () => {
    const hr = [...Array(450).fill(null), ...Array(300).fill(150)];
    expect(detectHeartRateIssues(samples(hr))).toEqual([{ kind: "missing", start: 0, end: 449 }]);
  });

  it("finds a spike that comes back", () => {
    const hr = Array(300).fill(145);
    for (let i = 150; i < 155; i++) hr[i] = 200;
    expect(detectHeartRateIssues(samples(hr))).toEqual([{ kind: "outlier", start: 150, end: 154 }]);
  });

  it("leaves a warm-up and a change of level alone", () => {
    const warmUp = Array.from({ length: 120 }, (_, i) => 90 + i * 0.5);
    const hr = [...warmUp, ...Array(200).fill(150), ...Array(200).fill(120)];
    expect(detectHeartRateIssues(samples(hr))).toEqual([]);
  });

  it("finds a strap settling in at the start", () => {
    const hr = [...Array(120).fill(185), ...Array(300).fill(140)];
    expect(detectHeartRateIssues(samples(hr)).map((i) => i.kind)).toContain("startArtifact");
  });

  it("finds a dropout", () => {
    const hr: (number | null)[] = Array(120).fill(145);
    for (let i = 60; i < 75; i++) hr[i] = null;
    expect(detectHeartRateIssues(samples(hr))).toEqual([{ kind: "dropout", start: 60, end: 74 }]);
  });
});

describe("detectStepsIssues", () => {
  it("finds a pod reading far too high at the start", () => {
    // The run of 16 Nov 2026: 280, 222, 208, then 186 and 170.
    const steps = [
      280,
      280,
      280,
      222,
      222,
      222,
      208,
      208,
      208,
      186,
      186,
      186,
      ...Array(200).fill(170),
    ];
    const issues = detectStepsIssues(samples(Array(212).fill(150), steps));
    expect(issues).toEqual([{ kind: "outlier", start: 0, end: 8 }]);
    const fixed = previewSampleEdits(samples(Array(212).fill(150), steps), [
      { channel: "steps", from: at(0), to: at(8), op: { kind: "interpolate" } },
    ]);
    // Leaning on the level after the stretch, not on the 186 right next to it.
    expect(fixed[0].steps).toBe(170);
  });

  it("finds steps missing while moving", () => {
    const steps: (number | null)[] = Array(200).fill(170);
    for (let i = 100; i < 115; i++) steps[i] = null;
    expect(detectStepsIssues(samples(Array(200).fill(140), steps))).toEqual([
      { kind: "dropout", start: 100, end: 114 },
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

describe("estimateStretch", () => {
  it("lets a missing start climb and meet the recording", () => {
    // 3 m/s throughout; the strap only connects after 450 s, at 150 bpm.
    const hr = [...Array(450).fill(null), ...Array(600).fill(150)];
    const track = samples(hr);
    const estimate = estimateStretch(track, "heartRate", 0, 449)!;
    expect(estimate).toHaveLength(450);
    expect(estimate[0]).toBeLessThan(115);
    expect(estimate[120]).toBeGreaterThan(140);
    expect(estimate[449]).toBe(150);
    // Never a flat line, never above the level it settles at.
    expect(new Set(estimate).size).toBeGreaterThan(10);
    expect(Math.max(...estimate)).toBeLessThanOrEqual(151);
  });

  it("needs enough recorded values to learn from", () => {
    const hr = [...Array(100).fill(null), ...Array(50).fill(150)];
    expect(estimateStretch(samples(hr), "heartRate", 0, 99)).toBeNull();
  });

  it("is applied as given", () => {
    const track = samples([140, 140, 140]);
    const fixed = previewSampleEdits(track, [
      { channel: "heartRate", from: at(0), to: at(1), op: { kind: "values", values: [120, 0] } },
    ]);
    expect(fixed.map((s) => s.heartRate)).toEqual([120, null, 140]);
  });
});
