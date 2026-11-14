/** Wording for the training effect score (1–12), from `ui/widget_trainings_effect.py`. */
const DESCRIPTIONS: Record<number, string> = {
  1: "Recovery – minimal training stimulus, good for recovery.",
  2: "Recovery – very light effort, maintains fitness but doesn’t improve it.",
  3: "Base – light training, improves basic endurance.",
  4: "Base – moderate training, beneficial for building endurance.",
  5: "Base – strong endurance training, prepares for harder sessions.",
  6: "Tempo – improves aerobic capacity and sustainable speed.",
  7: "Tempo – effective for increasing endurance performance.",
  8: "Threshold – high-intensity effort, enhances lactate threshold.",
  9: "Threshold – very strong training effect, challenging but sustainable.",
  10: "VO2 Max – near-maximal effort, improves oxygen utilization.",
  11: "VO2 Max – very high intensity, excellent for performance gains.",
  12: "Anaerobic – extreme effort, boosts speed and explosive power.",
};

export function effectDescription(score: number): string {
  return DESCRIPTIONS[score] ?? "Unknown training effect.";
}

/** "3 days" above 24 hours, otherwise whole hours. */
export function formatRecovery(hours: number): string {
  if (hours > 24) {
    const days = Math.floor(hours / 24);
    return `${days} day${days === 1 ? "" : "s"}`;
  }
  const whole = Math.floor(hours);
  return `${whole} hour${whole === 1 ? "" : "s"}`;
}
