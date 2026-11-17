import { describe, expect, it } from "vitest";

import type { TrainingEffect } from "@/bindings/TrainingEffect";

import { PACE_RAMP, rampColor, robustRange } from "./color-scale";
import {
  basisNotes,
  effectDescription,
  effectLabel,
  effectLevel,
  formatRecovery,
} from "./training-effect";

describe("colour scale", () => {
  it("interpolates and clamps", () => {
    expect(rampColor(PACE_RAMP, 0)).toBe("rgb(240 249 33)");
    expect(rampColor(PACE_RAMP, 1)).toBe("rgb(13 8 135)");
    expect(rampColor(PACE_RAMP, 2)).toBe(rampColor(PACE_RAMP, 1));
    expect(rampColor(PACE_RAMP, Number.NaN)).toBe(rampColor(PACE_RAMP, 0));
  });

  it("ignores outliers for the range", () => {
    const values = [...Array.from({ length: 98 }, (_, i) => 5 + i / 100), 1, 99];
    const [low, high] = robustRange(values);
    expect(low).toBeGreaterThan(4.9);
    expect(high).toBeLessThan(6);
    expect(robustRange([3, 3])).toEqual([2, 4]);
  });
});

describe("training effect wording", () => {
  it("labels the 0–5 scale and the recovery", () => {
    expect(effectLevel(0)).toBe(0);
    expect(effectLevel(0.96)).toBe(1);
    expect(effectLevel(2.9)).toBe(2);
    expect(effectLevel(3)).toBe(3);
    expect(effectLevel(7)).toBe(5);
    expect(effectLabel(0.4)).toBe("No effect");
    expect(effectLabel(3.4)).toBe("Improving");
    expect(effectLabel(4.9)).toBe("Highly improving");
    expect(effectLabel(5)).toBe("Overreaching");
    expect(formatRecovery(8.03)).toBe("8 hours");
    expect(formatRecovery(55)).toBe("2 days");
  });

  it("names every fallback for the heart rates and the fitness level", () => {
    const effect: TrainingEffect = {
      aerobic: 2.5,
      anaerobic: 0,
      peakEpoc: 22,
      recoveryHours: 17,
      maxHeartRate: 170,
      maxHeartRateSource: "age",
      restingHeartRate: 60,
      restingHeartRateSource: "default",
      vo2max: null,
      vo2maxSource: null,
      fromSplits: true,
    };
    const notes = basisNotes(effect);
    expect(notes[0]).toContain("220 − age");
    expect(notes[1]).toContain("assumed");
    expect(notes[2]).toContain("average fitness");
    expect(notes).toHaveLength(4);
    expect(
      basisNotes({
        ...effect,
        maxHeartRateSource: "profile",
        restingHeartRateSource: "profile",
        vo2max: 48.2,
        vo2maxSource: "runs",
        fromSplits: false,
      }),
    ).toEqual([
      "Maximum heart rate 170 bpm from your profile.",
      "Resting heart rate 60 bpm from your profile.",
      "Rated for your VO2max of 48.2, estimated from your runs.",
    ]);
  });
});

describe("effect score wording", () => {
  it("describes each score", () => {
    expect(effectDescription(1)).toMatch(/^Recovery/);
    expect(effectDescription(6)).toMatch(/^Tempo/);
    expect(effectDescription(12)).toMatch(/^Anaerobic/);
  });
});
