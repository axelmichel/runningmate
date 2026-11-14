import { describe, expect, it } from "vitest";

import { heatmapEnd, heatmapWeeks } from "./heatmap";

describe("heatmap", () => {
  const end = new Date(2026, 10, 14); // Saturday 14 Nov 2026

  it("covers 365 days in Monday-first weeks", () => {
    const weeks = heatmapWeeks([], end);
    expect(weeks.every((w) => w.length === 7)).toBe(true);
    expect(weeks[0][0].day).toBe("2025-11-10"); // Monday before 15 Nov 2025
    const inside = weeks.flat().filter((c) => !c.outside);
    expect(inside).toHaveLength(365);
    expect(inside.at(-1)?.day).toBe("2026-11-14");
  });

  it("scales levels to the busiest day", () => {
    const weeks = heatmapWeeks(
      [
        { day: "2026-11-10", duration: 3600 },
        { day: "2026-11-11", duration: 900 },
      ],
      end,
    );
    const cells = new Map(weeks.flat().map((c) => [c.day, c.level]));
    expect(cells.get("2026-11-10")).toBe(4);
    expect(cells.get("2026-11-11")).toBe(1);
    expect(cells.get("2026-11-12")).toBe(0);
  });

  it("picks the window end from the filter", () => {
    expect(heatmapEnd({ maxDate: 1_700_000_000 }).getTime()).toBe(1_700_000_000_000);
    expect(heatmapEnd({ minDate: 0 }).getTime()).toBe(365 * 86400 * 1000);
    expect(heatmapEnd({}, end)).toBe(end);
  });
});
