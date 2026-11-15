import { describe, expect, it } from "vitest";

import { zoneGradient } from "./zone-gradient";

const BOUNDS = [120, 140, 155, 170, 185];

describe("zoneGradient", () => {
  it("splits the line's range at the zone bounds, highest zone on top", () => {
    expect(zoneGradient(130, 160, BOUNDS)).toEqual([
      { zone: 4, from: 0, to: 5 / 30 },
      { zone: 3, from: 5 / 30, to: 20 / 30 },
      { zone: 2, from: 20 / 30, to: 1 },
    ]);
  });

  it("counts heart rates beyond the bounds to zones 1 and 5", () => {
    expect(zoneGradient(100, 110, BOUNDS)).toEqual([{ zone: 1, from: 0, to: 1 }]);
    expect(zoneGradient(190, 200, BOUNDS)).toEqual([{ zone: 5, from: 0, to: 1 }]);
  });

  it("is empty for a flat line", () => {
    expect(zoneGradient(150, 150, BOUNDS)).toEqual([]);
  });
});
