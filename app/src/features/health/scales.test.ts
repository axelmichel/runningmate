import { describe, expect, it } from "vitest";

import {
  FORM_SCALE,
  cardioScale,
  changeOf,
  formPercent,
  position,
  segmentOf,
  shareOf,
} from "./scales";

describe("health scales", () => {
  it("places VO2max in the age group's levels", () => {
    // Men aged 50–59.
    const scale = cardioScale([35.6, 39.2, 43.4, 48.9]);
    expect(scale.segments.map((s) => s.from)).toEqual([29, 35.6, 39.2, 43.4, 48.9]);
    expect(segmentOf(scale, 35.9).label).toBe("Fair");
    expect(segmentOf(scale, 20).label).toBe("Poor");
    expect(segmentOf(scale, 70).label).toBe("Superior");
  });

  it("rates form as a share of fitness", () => {
    expect(formPercent(-20, 100)).toBe(-20);
    expect(formPercent(5, 0)).toBeNull();
    expect(segmentOf(FORM_SCALE, -20).label).toBe("Optimal");
    expect(segmentOf(FORM_SCALE, -30).label).toBe("Optimal");
    expect(segmentOf(FORM_SCALE, 0).label).toBe("Grey zone");
    expect(segmentOf(FORM_SCALE, 80).label).toBe("Transition");
  });

  it("relates fitness to the personal best and the start of the period", () => {
    expect(shareOf(282.4, 307.6)).toBeCloseTo(91.8, 1);
    expect(shareOf(10, 0)).toBeNull();
    expect(changeOf(200, 250)).toBe(25);
    expect(changeOf(0, 50)).toBeNull();
  });

  it("clamps the marker to the scale", () => {
    expect(position(FORM_SCALE, -50)).toBe(0);
    expect(position(FORM_SCALE, -5)).toBe(50);
    expect(position(FORM_SCALE, 99)).toBe(100);
  });
});
