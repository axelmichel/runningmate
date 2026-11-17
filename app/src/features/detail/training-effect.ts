import type { TrainingEffect } from "@/bindings/TrainingEffect";
import { t, tk } from "@/i18n";
import { formatNumber } from "@/lib/format";

/** The usual wording of a training effect 0–5, one per whole step (5 is overreaching). */
const LABELS = [
  tk("No effect"),
  tk("Minor"),
  tk("Maintaining"),
  tk("Improving"),
  tk("Highly improving"),
  tk("Overreaching"),
] as const;

/** Whole step 0–5 of a training effect, after rounding to one decimal as it is shown. */
export function effectLevel(value: number): number {
  return Math.min(5, Math.max(0, Math.floor(Math.round(value * 10) / 10)));
}

export function effectLabel(value: number): string {
  return t(LABELS[effectLevel(value)]);
}

/** "3 days" above 24 hours, otherwise whole hours. */
export function formatRecovery(hours: number): string {
  if (hours > 24) {
    const days = Math.floor(hours / 24);
    return t("{{count}} day", { count: days });
  }
  const whole = Math.floor(hours);
  return t("{{count}} hour", { count: whole });
}

/** Where the heart rates and the fitness level come from, mentioning every fallback. */
export function basisNotes(effect: TrainingEffect): string[] {
  const max = { bpm: effect.maxHeartRate };
  const rest = { bpm: effect.restingHeartRate };
  const notes = [
    {
      profile: t("Maximum heart rate {{bpm}} bpm from your profile.", max),
      zone5: t(
        "Maximum heart rate {{bpm}} bpm: the upper bound of zone 5 (none in your profile).",
        max,
      ),
      age: t("Maximum heart rate {{bpm}} bpm: estimated as 220 − age (none in your profile).", max),
      default: t(
        "Maximum heart rate {{bpm}} bpm assumed: your profile has no maximum, zones or birthday.",
        max,
      ),
    }[effect.maxHeartRateSource],
    effect.restingHeartRateSource === "profile"
      ? t("Resting heart rate {{bpm}} bpm from your profile.", rest)
      : t("Resting heart rate {{bpm}} bpm assumed (none in your profile).", rest),
  ];
  const vo2max = { value: formatNumber(effect.vo2max, 1) };
  if (effect.vo2maxSource === "runs") {
    notes.push(t("Rated for your VO2max of {{value}}, estimated from your runs.", vo2max));
  } else if (effect.vo2maxSource === "profile") {
    notes.push(t("Rated for your VO2max of {{value}} from your profile.", vo2max));
  } else {
    notes.push(t("Rated for an average fitness level: no VO2max from runs or your profile."));
  }
  if (effect.fromSplits) {
    notes.push(t("Calculated from the split averages: the activity has no recorded track."));
  }
  return notes;
}
