import { describe, expect, it } from "vitest";

import { OPEN_RANGE, toDateRange } from "./range";

describe("toDateRange", () => {
  it("leaves empty fields open", () => {
    expect(toDateRange(OPEN_RANGE)).toEqual({});
  });

  it("spans whole local days", () => {
    const range = toDateRange({ from: "2025-05-01", to: "2025-05-06" });
    const from = new Date(range.from! * 1000);
    const to = new Date(range.to! * 1000);
    expect([from.getDate(), from.getHours(), from.getMinutes()]).toEqual([1, 0, 0]);
    expect([to.getDate(), to.getHours(), to.getMinutes()]).toEqual([6, 23, 59]);
  });
});
