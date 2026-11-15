import type { QueryClient } from "@tanstack/react-query";

import type { Goal } from "@/bindings/Goal";
import type { GoalInput } from "@/bindings/GoalInput";
import type { GoalMetric } from "@/bindings/GoalMetric";
import type { GoalPeriod } from "@/bindings/GoalPeriod";
import type { GoalProgress } from "@/bindings/GoalProgress";
import type { GoalSpec } from "@/bindings/GoalSpec";
import type { GoalWindow } from "@/bindings/GoalWindow";
import type { ViewMode } from "@/bindings/ViewMode";
import { localDay } from "@/features/dashboard/dashboard";
import type { Errors } from "@/features/settings/validation";
import { t, tk } from "@/i18n";
import { formatDate, formatDuration, formatHours, formatNumber, parseDuration } from "@/lib/format";

/** Longest window of a "days" goal, as in `src-tauri/src/db/goals.rs`. */
export const MAX_DAYS = 3650;

/** Labels are translation keys. */
export const METRICS: { metric: GoalMetric; label: string; unit: string }[] = [
  { metric: "distance", label: tk("Distance"), unit: "km" },
  { metric: "duration", label: tk("Time"), unit: "h" },
  { metric: "elevation", label: tk("Elevation"), unit: "m" },
  { metric: "count", label: tk("Activities"), unit: "" },
];

export const PERIODS: { period: GoalPeriod; label: string }[] = [
  { period: "week", label: tk("Week") },
  { period: "month", label: tk("Month") },
  { period: "year", label: tk("Year") },
  { period: "days", label: tk("Number of days") },
  { period: "open", label: tk("No period") },
];

/** The goal form as typed; `target` is in km, hours, m or activities. */
export type GoalForm = {
  name: string;
  view: ViewMode;
  metric: GoalMetric;
  target: string;
  perActivity: boolean;
  /** "H:MM:SS", optional. */
  maxDuration: string;
  period: GoalPeriod;
  days: string;
  repeat: boolean;
  /** "YYYY-MM-DD". */
  start: string;
  /** Runs for good; `end` is kept but not used. */
  endless: boolean;
  end: string;
};

export function emptyGoalForm(today = new Date()): GoalForm {
  return {
    name: "",
    view: "All",
    metric: "distance",
    target: "",
    perActivity: false,
    maxDuration: "",
    period: "week",
    days: "30",
    repeat: true,
    start: localDay(today),
    endless: true,
    end: "",
  };
}

export function goalForm(goal: Goal): GoalForm {
  const { spec } = goal;
  return {
    name: goal.name,
    view: goal.view,
    metric: spec.metric,
    target: String(spec.metric === "duration" ? spec.target / 3600 : spec.target),
    perActivity: spec.perActivity,
    maxDuration: spec.maxDuration ? formatDuration(spec.maxDuration) : "",
    period: spec.period,
    days: String(spec.days ?? 30),
    repeat: spec.repeat,
    start: goal.start,
    endless: goal.end === null,
    end: goal.end ?? "",
  };
}

const number = (value: string) => (value.trim() === "" ? Number.NaN : Number(value));
/** "YYYY-MM-DD" with a four-digit year; a date field gives "0026-…" for a year typed as 26. */
const isDate = (value: string) => /^\d{4}-\d{2}-\d{2}$/.test(value) && value >= "1900";

/** Whether the form allows one activity to reach the goal, and a time limit for it. */
export const allowsPerActivity = (form: GoalForm) => form.metric !== "count";
export const allowsTimeLimit = (form: GoalForm) =>
  form.perActivity && allowsPerActivity(form) && form.metric !== "duration";

/** Messages are translation keys. */
export function validateGoal(form: GoalForm): Errors<GoalForm> {
  const errors: Errors<GoalForm> = {};
  const target = number(form.target);
  if (!(target > 0)) errors.target = tk("Enter a target above zero");
  if (allowsTimeLimit(form) && form.maxDuration.trim() && !parseDuration(form.maxDuration))
    errors.maxDuration = tk("Use H:MM:SS");
  const days = number(form.days);
  if (form.period === "days" && !(Number.isInteger(days) && days >= 1 && days <= MAX_DAYS))
    errors.days = tk("Enter 1 to 3650 days");
  if (!isDate(form.start)) errors.start = tk("Pick a start date");
  if (!form.endless) {
    if (!isDate(form.end)) errors.end = tk("Pick an end date");
    else if (isDate(form.start) && form.end < form.start)
      errors.end = tk("The end can't be before the start");
  }
  return errors;
}

export function goalSpec(form: GoalForm): GoalSpec {
  const target = number(form.target);
  const perActivity = form.perActivity && allowsPerActivity(form);
  const maxDuration = allowsTimeLimit(form) ? parseDuration(form.maxDuration) : null;
  return {
    metric: form.metric,
    target: form.metric === "duration" ? Math.round(target * 3600) : target,
    perActivity,
    ...(maxDuration ? { maxDuration } : {}),
    period: form.period,
    ...(form.period === "days" ? { days: number(form.days) } : {}),
    repeat: form.repeat && form.period !== "open",
  };
}

/** What the backend stores; an empty name becomes the description. */
export function goalInput(form: GoalForm): GoalInput {
  const spec = goalSpec(form);
  return {
    name: form.name.trim() || describeGoal(spec),
    view: form.view,
    spec,
    start: form.start,
    end: form.endless ? null : form.end,
  };
}

/** Seconds → "5 h" for whole hours, otherwise "1 h 30 min". */
function formatTime(seconds: number): string {
  return seconds % 3600 === 0 ? `${formatNumber(seconds / 3600)} h` : formatHours(seconds);
}

/** A value of the metric with its unit, e.g. "12.5 km". */
export function formatGoalValue(metric: GoalMetric, value: number): string {
  switch (metric) {
    case "distance": {
      const tenths = value * 10;
      const digits = Number.isInteger(value)
        ? 0
        : Math.abs(tenths - Math.round(tenths)) < 1e-6
          ? 1
          : 2;
      return `${formatNumber(value, digits)} km`;
    }
    case "duration":
      return formatTime(value);
    case "elevation":
      return `${formatNumber(value)} m`;
    case "count":
      return formatNumber(value);
  }
}

/** "100 km every month", "10 km in under 0:50:00 in one activity", "3 activities every week". */
export function describeGoal(spec: GoalSpec): string {
  const amount =
    spec.metric === "count"
      ? t("{{count}} activities", { count: spec.target })
      : spec.metric === "elevation"
        ? t("{{value}} climbed", { value: formatGoalValue("elevation", spec.target) })
        : formatGoalValue(spec.metric, spec.target);
  const parts = [amount];
  if (spec.perActivity) {
    if (spec.maxDuration)
      parts.push(t("in under {{time}}", { time: formatDuration(spec.maxDuration) }));
    parts.push(t("in one activity"));
  }
  const count = spec.days ?? 1;
  const period: Record<GoalPeriod, string> = spec.repeat
    ? {
        week: t("every week"),
        month: t("every month"),
        year: t("every year"),
        days: t("every {{count}} days", { count }),
        open: "",
      }
    : {
        week: t("in a week"),
        month: t("in a month"),
        year: t("in a year"),
        days: t("in {{count}} days", { count }),
        open: "",
      };
  parts.push(period[spec.period]);
  return parts.filter(Boolean).join(" ");
}

/** Share of the target reached in the window, 0–100. */
export function windowPercent(spec: GoalSpec, window: GoalWindow): number {
  if (window.achievedAt !== null) return 100;
  return Math.max(0, Math.min(100, (window.value / spec.target) * 100));
}

/** "3 Nov 2025 – 9 Nov 2025", "2025", or "Since 1 Nov 2025" for an open window. */
export function windowLabel(spec: GoalSpec, window: GoalWindow): string {
  if (window.end === null) return t("Since {{date}}", { date: formatDate(window.start) });
  if (spec.period === "year") return String(new Date(window.start * 1000).getFullYear());
  const last = window.end - 1;
  return `${formatDate(window.start)} – ${formatDate(last)}`;
}

/** Whole days until the window ends, at least one while it runs; `null` when open or over. */
export function daysLeft(window: GoalWindow, now: number): number | null {
  if (window.end === null || window.end <= now) return null;
  return Math.ceil((window.end - now) / 86400);
}

/** Short state line under a goal: reached, days left, upcoming, missed or ended. */
export function goalHint(progress: GoalProgress, now: number): string | null {
  const { current, status, goal } = progress;
  switch (status) {
    case "upcoming":
      return t("Starts {{date}}", { date: formatDate(current.start) });
    case "achieved":
      return t("Reached {{date}}", { date: formatDate(current.achievedAt) });
    case "missed":
      return t("Missed");
    case "ended":
      return t("Ended");
    case "active": {
      if (current.achievedAt !== null)
        return t("Reached {{date}}", { date: formatDate(current.achievedAt) });
      const days = daysLeft(current, now);
      if (days === null) return null;
      const missing = goal.spec.perActivity ? null : goal.spec.target - current.value;
      const left = t("{{count}} days left", { count: days });
      return missing && missing > 0
        ? `${t("{{value}} to go", { value: formatGoalValue(goal.spec.metric, missing) })} · ${left}`
        : left;
    }
  }
}

/** Everything that shows goals or the stars of activities. */
export function invalidateGoals(queryClient: QueryClient) {
  void queryClient.invalidateQueries({ queryKey: ["goals"] });
  void queryClient.invalidateQueries({ queryKey: ["goalHits"] });
}
