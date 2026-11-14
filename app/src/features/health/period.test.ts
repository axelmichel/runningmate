import { describe, expect, it } from "vitest";

import { lastMonths, periodBounds } from "./period";

describe("health period", () => {
  it("covers the last months up to today", () => {
    expect(lastMonths(3, new Date(2026, 10, 14))).toEqual({ from: "2026-08-15", to: "2026-11-14" });
    expect(lastMonths(1, new Date(2026, 0, 31))).toEqual({ from: "2026-01-01", to: "2026-01-31" });
  });

  it("needs both dates in order", () => {
    expect(periodBounds({ from: "", to: "2026-11-14" })).toBeNull();
    expect(periodBounds({ from: "2026-11-15", to: "2026-11-14" })).toBeNull();
    const bounds = periodBounds({ from: "2026-11-14", to: "2026-11-14" });
    expect(bounds && bounds.to - bounds.from).toBe(86_399);
  });
});
