import { describe, expect, it } from "vitest";

import { customFloor, zoneRanges } from "./zones";

describe("zoneRanges", () => {
  it("starts each zone above the previous bound", () => {
    expect(zoneRanges(117, [131, 144, 158, 171, 185])).toEqual([
      [117, 131],
      [132, 144],
      [145, 158],
      [159, 171],
      [172, 185],
    ]);
  });

  it("starts custom zones at half of the zone 5 bound", () => {
    expect(customFloor([98, 114, 131, 147, 165])).toBe(82);
    expect(zoneRanges(customFloor([98, 114, 131, 147, 164]), [98, 114, 131, 147, 164])[0]).toEqual([
      82, 98,
    ]);
  });
});
