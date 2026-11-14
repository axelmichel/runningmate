import { t, tk } from "@/i18n";

/** Wording for the training effect score (1–12), from `ui/widget_trainings_effect.py`. */
const DESCRIPTIONS: Record<number, string> = {
  1: tk("Recovery – minimal training stimulus, good for recovery."),
  2: tk("Recovery – very light effort, maintains fitness but doesn’t improve it."),
  3: tk("Base – light training, improves basic endurance."),
  4: tk("Base – moderate training, beneficial for building endurance."),
  5: tk("Base – strong endurance training, prepares for harder sessions."),
  6: tk("Tempo – improves aerobic capacity and sustainable speed."),
  7: tk("Tempo – effective for increasing endurance performance."),
  8: tk("Threshold – high-intensity effort, enhances lactate threshold."),
  9: tk("Threshold – very strong training effect, challenging but sustainable."),
  10: tk("VO2 Max – near-maximal effort, improves oxygen utilization."),
  11: tk("VO2 Max – very high intensity, excellent for performance gains."),
  12: tk("Anaerobic – extreme effort, boosts speed and explosive power."),
};

export function effectDescription(score: number): string {
  return t(DESCRIPTIONS[score] ?? tk("Unknown training effect."));
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
