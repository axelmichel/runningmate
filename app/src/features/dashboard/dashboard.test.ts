import { describe, expect, it } from "vitest";

import type { ActivityRow } from "@/bindings/ActivityRow";

import {
  byDay,
  calendarWeeks,
  chartBars,
  chartStarts,
  circleSize,
  localDay,
  perWeek,
  tileSpan,
  yearStart,
} from "./dashboard";

describe("dashboard", () => {
  it("shows the current and the three previous weeks, Monday first", () => {
    // Saturday, 14 November 2026.
    const weeks = calendarWeeks(new Date(2026, 10, 14, 18, 30));
    expect(weeks).toHaveLength(4);
    expect(weeks.every((week) => week.length === 7)).toBe(true);
    expect(localDay(weeks[0][0])).toBe("2026-10-19");
    expect(weeks[0][0].getDay()).toBe(1);
    expect(localDay(weeks[3][6])).toBe("2026-11-15");
  });

  it("groups activities by local day", () => {
    const row = (id: number, date: Date) =>
      ({ id, date: Math.floor(date.getTime() / 1000) }) as ActivityRow;
    const days = byDay([
      row(1, new Date(2026, 10, 14, 7)),
      row(2, new Date(2026, 10, 14, 19)),
      row(3, new Date(2026, 10, 13, 12)),
    ]);
    expect(days.get("2026-11-14")?.map((r) => r.id)).toEqual([1, 2]);
    expect(days.get("2026-11-13")?.map((r) => r.id)).toEqual([3]);
  });

  it("sizes circles by the area, relative to the longest day", () => {
    expect(circleSize(0, 3600)).toBe(0);
    expect(circleSize(3600, 3600)).toBe(1);
    expect(circleSize(900, 3600)).toBeCloseTo(0.3 + 0.7 * 0.5);
    expect(circleSize(10, 3600)).toBeGreaterThan(0.3);
  });

  it("averages per week and starts the year on 1 January", () => {
    expect(perWeek({ count: 10, distance: 40, duration: 14400, elevationGain: 400 })).toEqual({
      count: 2.5,
      distance: 10,
      duration: 3600,
      elevationGain: 100,
    });
    expect(new Date(yearStart(new Date(2026, 10, 14)) * 1000)).toEqual(new Date(2026, 0, 1));
  });
});

describe("bar chart", () => {
  const today = new Date(2026, 10, 14, 18);
  const row = (date: Date, viewMode: ActivityRow["viewMode"], distance: number) =>
    ({
      id: 1,
      date: Math.floor(date.getTime() / 1000),
      viewMode,
      distance,
      duration: 3600,
      elevationGain: 50,
    }) as ActivityRow;

  it("starts the bars on Mondays or on the first of the month", () => {
    const weeks = chartStarts("week", today);
    expect(weeks).toHaveLength(12);
    expect(localDay(weeks[11])).toBe("2026-11-09");
    expect(localDay(weeks[0])).toBe("2026-08-24");
    const months = chartStarts("month", today);
    expect(localDay(months[11])).toBe("2026-11-01");
    expect(localDay(months[0])).toBe("2025-12-01");
  });

  it("stacks the sports per week and month and drops older activities", () => {
    const rows = [
      row(new Date(2026, 10, 9, 7), "Running", 10),
      row(new Date(2026, 10, 14, 9), "Cycling", 40),
      row(new Date(2026, 10, 8, 9), "Walking", 5),
      row(new Date(2026, 0, 1, 9), "Running", 99),
      row(new Date(2026, 10, 10, 9), "All", 7),
    ];
    const weeks = chartBars(rows, "week", "distance", today);
    expect([weeks[11].Running, weeks[11].Cycling, weeks[11].total]).toEqual([10, 40, 50]);
    expect(weeks[10].Walking).toBe(5);
    expect(weeks.reduce((sum, bar) => sum + bar.total, 0)).toBe(55);

    const months = chartBars(rows, "month", "duration", today);
    expect(months[11].total).toBe(3);
    expect(months[1].Running).toBe(1);
    expect(chartBars(rows, "month", "elevation", today)[11].Walking).toBe(50);
  });
});

describe("photo tiles", () => {
  const spans = (count: number) => Array.from({ length: count }, (_, i) => tileSpan(i, count));

  it("gives the first photo the left half and shares the right half among the rest", () => {
    expect(spans(1)).toEqual(["col-span-4 row-span-2"]);
    expect(spans(2)).toEqual(["col-span-2 row-span-2", "col-span-2 row-span-2"]);
    expect(spans(3)).toEqual(["col-span-2 row-span-2", "col-span-2", "col-span-2"]);
    expect(spans(4)).toEqual(["col-span-2 row-span-2", "", "", "col-span-2"]);
    expect(spans(5)).toEqual(["col-span-2 row-span-2", "", "", "", ""]);
  });
});
