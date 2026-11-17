import { describe, expect, it } from "vitest";

import type { GoalProgress } from "@/bindings/GoalProgress";
import type { GoalSpec } from "@/bindings/GoalSpec";

import {
  daysLeft,
  describeGoal,
  emptyGoalForm,
  formatGoalValue,
  goalForm,
  goalHint,
  goalInput,
  validateGoal,
  windowPercent,
  type GoalForm,
} from "./goal-math";

const form = (change: Partial<GoalForm>): GoalForm => ({
  ...emptyGoalForm(new Date(2025, 10, 3)),
  target: "100",
  ...change,
});

const spec = (change: Partial<GoalSpec>): GoalSpec => ({
  metric: "distance",
  target: 100,
  perActivity: false,
  period: "month",
  repeat: true,
  ...change,
});

describe("goal form", () => {
  it("starts today with a repeating weekly total", () => {
    const empty = emptyGoalForm(new Date(2025, 10, 3));
    expect(empty).toMatchObject({ start: "2025-11-03", period: "week", repeat: true });
  });

  it("reports invalid fields", () => {
    expect(validateGoal(form({}))).toEqual({});
    expect(Object.keys(validateGoal(form({ target: "0" })))).toEqual(["target"]);
    expect(Object.keys(validateGoal(form({ period: "days", days: "0" })))).toEqual(["days"]);
    expect(Object.keys(validateGoal(form({ endless: false, end: "2025-11-02" })))).toEqual(["end"]);
    // A month from its first to its last day; a year typed with two digits is not a date yet.
    expect(validateGoal(form({ start: "2025-11-01", endless: false, end: "2025-11-30" }))).toEqual(
      {},
    );
    expect(Object.keys(validateGoal(form({ endless: false, end: "0025-11-30" })))).toEqual(["end"]);
    expect(Object.keys(validateGoal(form({ endless: false, end: "" })))).toEqual(["end"]);
    // Without an end date the end field doesn't count.
    expect(validateGoal(form({ endless: true, end: "2025-11-02" }))).toEqual({});
    expect(Object.keys(validateGoal(form({ start: "" })))).toEqual(["start"]);
    expect(Object.keys(validateGoal(form({ perActivity: true, maxDuration: "fast" })))).toEqual([
      "maxDuration",
    ]);
    // The time limit is ignored where it doesn't apply.
    expect(validateGoal(form({ perActivity: false, maxDuration: "fast" }))).toEqual({});
  });

  it("turns the form into what the backend stores", () => {
    expect(
      goalInput(form({ metric: "duration", target: "1.5", period: "days", days: "10" })),
    ).toEqual({
      name: "1 h 30 min every 10 days",
      view: "All",
      spec: {
        metric: "duration",
        target: 5400,
        perActivity: false,
        period: "days",
        days: 10,
        repeat: true,
      },
      start: "2025-11-03",
      end: null,
    });
    const fast = goalInput(
      form({
        name: " 10 k ",
        view: "Running",
        target: "10",
        perActivity: true,
        maxDuration: "0:50:00",
        period: "open",
        endless: false,
        end: "2025-12-31",
      }),
    );
    expect(fast.name).toBe("10 k");
    expect(fast.spec).toEqual({
      metric: "distance",
      target: 10,
      perActivity: true,
      maxDuration: 3000,
      period: "open",
      repeat: false,
    });
    expect(fast.end).toBe("2025-12-31");
    expect(goalInput(form({ period: "open", end: "2025-12-31" })).end).toBeNull();
    // A number of activities is always a total.
    expect(
      goalInput(form({ metric: "count", target: "3", perActivity: true })).spec.perActivity,
    ).toBe(false);
  });

  it("round-trips a stored goal", () => {
    const input = goalInput(form({ metric: "duration", target: "5", maxDuration: "" }));
    const back = goalForm({ id: 1, archived: false, created: 0, ...input });
    expect(goalInput(back)).toEqual(input);
  });
});

describe("goal texts", () => {
  it("formats values with their unit", () => {
    expect(formatGoalValue("distance", 100)).toBe("100 km");
    expect(formatGoalValue("distance", 21.1)).toBe("21.1 km");
    expect(formatGoalValue("distance", 42.195)).toBe("42.20 km");
    expect(formatGoalValue("duration", 18000)).toBe("5 h");
    expect(formatGoalValue("duration", 5400)).toBe("1 h 30 min");
    expect(formatGoalValue("elevation", 1000)).toBe("1,000 m");
    expect(formatGoalValue("count", 3)).toBe("3");
  });

  it("describes a goal", () => {
    expect(describeGoal(spec({}))).toBe("100 km every month");
    expect(describeGoal(spec({ metric: "count", target: 3, period: "week" }))).toBe(
      "3 activities every week",
    );
    expect(
      describeGoal(
        spec({ target: 10, perActivity: true, maxDuration: 3000, period: "open", repeat: false }),
      ),
    ).toBe("10 km in under 0:50:00 in one activity");
    expect(describeGoal(spec({ metric: "elevation", target: 500, period: "days", days: 1 }))).toBe(
      "500 m climbed every day",
    );
    expect(describeGoal(spec({ period: "days", days: 30, repeat: false }))).toBe(
      "100 km in 30 days",
    );
  });
});

describe("goal progress", () => {
  const window = {
    start: 0,
    end: 7 * 86400,
    value: 40,
    activities: 2,
    achievedAt: null,
    achievedActivityId: null,
  };

  it("caps the share of the target", () => {
    expect(windowPercent(spec({}), window)).toBe(40);
    expect(windowPercent(spec({}), { ...window, value: 140 })).toBe(100);
    expect(windowPercent(spec({}), { ...window, achievedAt: 10 })).toBe(100);
  });

  it("counts the days left", () => {
    expect(daysLeft(window, 0)).toBe(7);
    expect(daysLeft(window, 6 * 86400 + 1)).toBe(1);
    expect(daysLeft(window, 7 * 86400)).toBeNull();
    expect(daysLeft({ ...window, end: null }, 0)).toBeNull();
  });

  it("says what is left to do", () => {
    const progress: GoalProgress = {
      goal: {
        id: 1,
        name: "Monthly",
        view: "All",
        spec: spec({}),
        start: "2025-11-01",
        end: null,
        archived: false,
        created: 0,
      },
      status: "active",
      current: window,
      history: [],
      achieved: 0,
      windows: 0,
      streak: 0,
      hits: 0,
      lastHit: null,
      bestDuration: null,
    };
    expect(goalHint(progress, 5 * 86400)).toBe("60 km to go · 2 days left");
    expect(goalHint({ ...progress, status: "missed" }, 0)).toBe("Missed");
  });
});
