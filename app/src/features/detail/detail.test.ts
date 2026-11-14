import { describe, expect, it } from "vitest";

import { PACE_RAMP, rampColor, robustRange } from "./color-scale";
import { effectDescription, formatRecovery } from "./training-effect";

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
  it("describes scores and recovery", () => {
    expect(effectDescription(6)).toMatch(/^Tempo/);
    expect(formatRecovery(8.03)).toBe("8 hours");
    expect(formatRecovery(55)).toBe("2 days");
  });
});
