import { describe, expect, it } from "vitest";

import {
  addDays,
  emptyPlanForm,
  nextMonday,
  planInput,
  sessionDetail,
  structureLabel,
  targetLabel,
  validatePlan,
  weekdayOf,
} from "./training-math";

describe("training math", () => {
  it("starts a new plan on the next Monday", () => {
    // Sunday 15 November 2026, Monday 16 November.
    expect(nextMonday(new Date(2026, 10, 15))).toBe("2026-11-16");
    expect(nextMonday(new Date(2026, 10, 16))).toBe("2026-11-16");
    expect(nextMonday(new Date(2026, 10, 17))).toBe("2026-11-23");
  });

  it("counts days and weekdays in local time", () => {
    expect(addDays("2026-12-30", 3)).toBe("2027-01-02");
    expect(addDays("2026-03-01", -1)).toBe("2026-02-28");
    expect(weekdayOf("2026-11-16")).toBe(0);
    expect(weekdayOf("2026-11-22")).toBe(6);
  });

  it("turns the form into a plan input", () => {
    const form = { ...emptyPlanForm(new Date(2026, 10, 15)), race: "half" as const };
    expect(planInput({ ...form, targetTime: "1:45:00", days: [6, 1, 3] })).toEqual({
      name: "Half marathon in 1:45:00",
      target: "race",
      distance: 21.0975,
      targetTime: 6300,
      start: "2026-11-16",
      weeks: 8,
      days: [1, 3, 6],
      longDay: 6,
    });
    expect(planInput({ ...form, target: "shape", name: " Winter " })).toMatchObject({
      name: "Winter",
      distance: null,
      targetTime: null,
    });
    expect(planInput({ ...form, race: "custom", distance: "15,5" }).distance).toBe(15.5);
  });

  it("checks the form", () => {
    const form = emptyPlanForm(new Date(2026, 10, 15));
    expect(validatePlan(form)).toEqual({});
    expect(Object.keys(validatePlan({ ...form, race: "custom", distance: "" }))).toEqual([
      "distance",
    ]);
    expect(Object.keys(validatePlan({ ...form, targetTime: "fast" }))).toEqual(["targetTime"]);
    expect(Object.keys(validatePlan({ ...form, weeks: "0" }))).toEqual(["weeks"]);
    expect(Object.keys(validatePlan({ ...form, days: [1, 3] }))).toEqual(["longDay"]);
    expect(Object.keys(validatePlan({ ...form, days: [] }))).toEqual(["days"]);
  });

  it("describes sessions", () => {
    expect(structureLabel({ reps: 6, metres: 800, rest: 120 })).toBe("6 × 800 m");
    expect(structureLabel({ reps: 5, metres: 1000, rest: 180 })).toBe("5 × 1 km");
    expect(structureLabel({ reps: 1, seconds: 1200, rest: 0 })).toBe("20 min");
    expect(
      sessionDetail({
        distance: 9,
        pace: { fast: 230, slow: 236 },
        structure: { reps: 5, metres: 1000, rest: 180 },
      }),
    ).toBe("9 km · 5 × 1 km @ 3:50–3:56 /km");
    expect(sessionDetail({ distance: 7.5, pace: null, structure: null })).toBe("7.5 km");
  });

  it("names the target", () => {
    expect(targetLabel({ target: "race", distance: 10, targetTime: null })).toBe("10 km");
    expect(targetLabel({ target: "race", distance: 42.195, targetTime: 12600 })).toBe(
      "Marathon in 3:30:00",
    );
    expect(targetLabel({ target: "fitness", distance: null, targetTime: null })).toBe(
      "More fitness",
    );
  });
});
