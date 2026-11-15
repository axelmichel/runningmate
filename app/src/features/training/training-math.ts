import type { QueryClient } from "@tanstack/react-query";

import type { PaceRange } from "@/bindings/PaceRange";
import type { Phase } from "@/bindings/Phase";
import type { PlanInput } from "@/bindings/PlanInput";
import type { PlanWarning } from "@/bindings/PlanWarning";
import type { SessionKind } from "@/bindings/SessionKind";
import type { SessionStatus } from "@/bindings/SessionStatus";
import type { Structure } from "@/bindings/Structure";
import type { TrainingPlan } from "@/bindings/TrainingPlan";
import type { TrainingSession } from "@/bindings/TrainingSession";
import type { TrainingTarget } from "@/bindings/TrainingTarget";
import type { Verdict } from "@/bindings/Verdict";
import { localDay, parseLocalDay } from "@/features/dashboard/dashboard";
import i18n, { t, tk } from "@/i18n";
import { formatDuration, formatNumber, formatPace, parseDuration } from "@/lib/format";

export const TARGETS: { target: TrainingTarget; label: string; hint: string }[] = [
  { target: "race", label: tk("A race"), hint: tk("5 km, 10 km, half marathon, marathon") },
  { target: "shape", label: tk("Get in shape"), hint: tk("Mostly easy runs, one faster run") },
  { target: "fitness", label: tk("More fitness"), hint: tk("Intervals and tempo runs") },
];

export type RaceChoice = "5" | "10" | "half" | "marathon" | "custom";

export const RACES: { race: RaceChoice; km?: number; label: string }[] = [
  { race: "5", km: 5, label: "5 km" },
  { race: "10", km: 10, label: "10 km" },
  { race: "half", km: 21.0975, label: tk("Half marathon") },
  { race: "marathon", km: 42.195, label: tk("Marathon") },
  { race: "custom", label: tk("Other distance") },
];

/** Plan lengths offered, in weeks. */
export const LENGTHS = [4, 6, 8, 12, 16];

export const KIND_LABELS: Record<SessionKind, string> = {
  easy: tk("Easy run"),
  recovery: tk("Recovery run"),
  long: tk("Long run"),
  tempo: tk("Tempo run"),
  intervals: tk("Intervals"),
  race: tk("Race"),
};

export const KINDS = Object.keys(KIND_LABELS) as SessionKind[];

/** Colours of the session kinds, from calm to hard. */
export const KIND_COLORS: Record<SessionKind, string> = {
  easy: "var(--chart-5)",
  recovery: "var(--chart-5)",
  long: "var(--chart-4)",
  tempo: "var(--chart-3)",
  intervals: "var(--chart-2)",
  race: "var(--chart-1)",
};

export const PHASE_LABELS: Record<Phase, string> = {
  base: tk("Base"),
  build: tk("Build"),
  peak: tk("Peak"),
  taper: tk("Taper"),
  recovery: tk("Easier week"),
};

export const VERDICT_LABELS: Record<Verdict, string> = {
  planned: tk("Planned"),
  done: tk("Done as planned"),
  partial: tk("Partly done"),
  tooHard: tk("Harder than planned"),
  tooEasy: tk("Easier than planned"),
  missed: tk("Missed"),
  skipped: tk("Skipped"),
};

/** Text colours of the verdicts. */
export const VERDICT_COLORS: Record<Verdict, string> = {
  planned: "text-muted-foreground",
  done: "text-emerald-600 dark:text-emerald-400",
  partial: "text-amber-600 dark:text-amber-400",
  tooHard: "text-orange-600 dark:text-orange-400",
  tooEasy: "text-sky-600 dark:text-sky-400",
  missed: "text-destructive",
  skipped: "text-muted-foreground",
};

/** Ring colours of the verdicts in the calendar. */
export const VERDICT_RINGS: Record<Verdict, string> = {
  planned: "var(--muted-foreground)",
  done: "#059669",
  partial: "#d97706",
  tooHard: "#ea580c",
  tooEasy: "#0284c7",
  missed: "var(--destructive)",
  skipped: "var(--muted-foreground)",
};

/** Short weekday names, Monday (0) first. */
export function weekdayNames(style: "narrow" | "short" | "long" = "short"): string[] {
  const format = new Intl.DateTimeFormat(i18n.language, { weekday: style });
  // 1 January 2024 was a Monday.
  return Array.from({ length: 7 }, (_, i) => format.format(new Date(2024, 0, 1 + i)));
}

/** "YYYY-MM-DD" `days` later (or earlier). */
export function addDays(day: string, days: number): string {
  const date = parseLocalDay(day) ?? new Date();
  return localDay(new Date(date.getFullYear(), date.getMonth(), date.getDate() + days));
}

/** Weekday of a "YYYY-MM-DD", 0 = Monday. */
export function weekdayOf(day: string): number {
  return ((parseLocalDay(day)?.getDay() ?? 1) + 6) % 7;
}

/** Seconds per km → "M:SS". */
export function formatPaceSeconds(seconds: number | null | undefined): string {
  return formatPace(seconds == null ? null : seconds / 60);
}

export function paceRangeLabel(range: PaceRange): string {
  return `${formatPaceSeconds(range.fast)}–${formatPaceSeconds(range.slow)} /km`;
}

/** "6 × 800 m" or "3 × 10 min". */
export function structureLabel(structure: Structure): string {
  const each =
    structure.metres != null
      ? structure.metres >= 1000 && structure.metres % 1000 === 0
        ? `${formatNumber(structure.metres / 1000)} km`
        : `${formatNumber(structure.metres)} m`
      : `${formatNumber((structure.seconds ?? 0) / 60)} min`;
  return structure.reps > 1 ? `${structure.reps} × ${each}` : each;
}

/** What a session asks for: "8 km · 5:10–5:40 /km" or "9 km · 5 × 1 km @ 3:50–3:56 /km". */
export function sessionDetail(session: Pick<TrainingSession, "distance" | "pace" | "structure">) {
  const km = `${formatNumber(session.distance, session.distance % 1 === 0 ? 0 : 1)} km`;
  const pace = session.pace ? paceRangeLabel(session.pace) : null;
  if (session.structure) {
    const work = structureLabel(session.structure);
    return [km, pace ? `${work} @ ${pace}` : work].join(" · ");
  }
  return pace ? `${km} · ${pace}` : km;
}

/** What the plan aims for: "10 km in 0:45:00", "Half marathon", "Get in shape". */
export function targetLabel(plan: Pick<TrainingPlan, "target" | "distance" | "targetTime">) {
  if (plan.target !== "race") {
    return t(TARGETS.find((x) => x.target === plan.target)?.label ?? plan.target);
  }
  const race = RACES.find(
    (r) => r.km !== undefined && Math.abs(r.km - (plan.distance ?? 0)) < 0.05,
  );
  const distance = race ? t(race.label) : `${formatNumber(plan.distance, 1)} km`;
  return plan.targetTime
    ? t("{{distance}} in {{time}}", { distance, time: formatDuration(plan.targetTime) })
    : distance;
}

export function warningText(warning: PlanWarning): string {
  switch (warning.code) {
    case "tooShort":
      return t("Plans for this distance usually take {{weeks}} weeks or more.", {
        weeks: warning.weeks,
      });
    case "lowVolume":
      return t(
        "You ran little lately; this distance usually builds on about {{km}} km a week. The plan starts gently.",
        { km: formatNumber(warning.km) },
      );
    case "noPaces":
      return t(
        "There are too few runs to work out paces, so the sessions only have distances. Enter a target time or your VO2max in the profile.",
      );
    case "ambitious":
      return t(
        "The target time needs a VO2max of about {{vdot}}, well above your current one. Training starts from where you are.",
        { vdot: formatNumber(warning.vdot) },
      );
  }
}

/** One line on how the run did against the session. */
export function checkText(status: SessionStatus): string {
  const { check, run, session } = status;
  const parts = [t(VERDICT_LABELS[check.verdict])];
  if (run) {
    parts.push(
      `${formatNumber(run.distance, 1)} / ${formatNumber(session.distance, 1)} km`,
      `${formatPaceSeconds(run.pace)} /km`,
    );
    if (check.workPace != null) {
      parts.push(t("fastest km {{pace}} /km", { pace: formatPaceSeconds(check.workPace) }));
    }
    if (check.aboveZoneTwo) parts.push(t("heart rate above zone 2"));
  }
  return parts.join(" · ");
}

/** What the plan wizard edits; numbers stay text until saved. */
export type PlanForm = {
  name: string;
  target: TrainingTarget;
  race: RaceChoice;
  /** km, for another distance. */
  distance: string;
  /** "H:MM:SS", optional. */
  targetTime: string;
  start: string;
  weeks: string;
  days: number[];
  longDay: number;
  createGoal: boolean;
};

/** The Monday after `today`, or `today` when it is a Monday. */
export function nextMonday(today = new Date()): string {
  const offset = (8 - today.getDay()) % 7;
  return localDay(new Date(today.getFullYear(), today.getMonth(), today.getDate() + offset));
}

export function emptyPlanForm(today = new Date()): PlanForm {
  return {
    name: "",
    target: "race",
    race: "10",
    distance: "",
    targetTime: "",
    start: nextMonday(today),
    weeks: "8",
    days: [1, 3, 6],
    longDay: 6,
    createGoal: false,
  };
}

function raceKm(form: PlanForm): number | null {
  if (form.target !== "race") return null;
  const preset = RACES.find((r) => r.race === form.race)?.km;
  if (preset !== undefined) return preset;
  const km = Number(form.distance.replace(",", "."));
  return form.distance.trim() && Number.isFinite(km) ? km : NaN;
}

export function validatePlan(form: PlanForm): Partial<Record<keyof PlanForm, string>> {
  const errors: Partial<Record<keyof PlanForm, string>> = {};
  const km = raceKm(form);
  if (km !== null && !(km >= 1 && km <= 100)) {
    errors.distance = tk("Enter a distance between 1 and 100 km");
  }
  if (form.target === "race" && form.targetTime.trim() && !parseDuration(form.targetTime)) {
    errors.targetTime = tk("Enter the time as H:MM:SS");
  }
  if (!parseLocalDay(form.start)) errors.start = tk("Pick a start date");
  const weeks = Number(form.weeks);
  if (!Number.isInteger(weeks) || weeks < 1 || weeks > 52) {
    errors.weeks = tk("Between 1 and 52 weeks");
  }
  if (form.days.length === 0) errors.days = tk("Pick at least one day");
  else if (!form.days.includes(form.longDay)) {
    errors.longDay = tk("The long run must be on a training day");
  }
  return errors;
}

/** A name from the target, for when none is given. */
export function suggestedName(form: PlanForm): string {
  const km = raceKm(form);
  const time = form.target === "race" ? parseDuration(form.targetTime) : null;
  return targetLabel({
    target: form.target,
    distance: km !== null && Number.isFinite(km) ? km : null,
    targetTime: time,
  });
}

export function planInput(form: PlanForm): PlanInput {
  const km = raceKm(form);
  return {
    name: form.name.trim() || suggestedName(form),
    target: form.target,
    distance: km,
    targetTime: form.target === "race" ? parseDuration(form.targetTime) : null,
    start: form.start,
    weeks: Number(form.weeks),
    days: [...form.days].sort((a, b) => a - b),
    longDay: form.longDay,
  };
}

export function invalidateTraining(queryClient: QueryClient) {
  void queryClient.invalidateQueries({ queryKey: ["training"] });
}
