import type { CardioLevel } from "@/bindings/CardioLevel";
import { tk } from "@/i18n";

/** A stretch of a scale; `label` and `description` are translation keys. */
export type Segment = {
  label: string;
  description?: string;
  from: number;
  to: number;
  color: string;
};

export type Scale = { segments: Segment[]; min: number; max: number };

export const CARDIO_LEVELS: Record<CardioLevel, string> = {
  poor: tk("Poor"),
  fair: tk("Fair"),
  good: tk("Good"),
  excellent: tk("Excellent"),
  superior: tk("Superior"),
};

/** Light to dark: the darker, the fitter. */
const CARDIO_COLORS = ["#f6b48f", "#f37651", "#e13342", "#ad1759", "#701f57"];

/** VO2max scale for an age group from the lower bounds of Fair, Good, Excellent, Superior. */
export function cardioScale(bounds: [number, number, number, number]): Scale {
  const min = Math.floor(bounds[0] - 6);
  const max = Math.ceil(bounds[3] + 6);
  const edges = [min, ...bounds, max];
  const labels = Object.values(CARDIO_LEVELS);
  return {
    min,
    max,
    segments: labels.map((label, i) => ({
      label,
      from: edges[i],
      to: edges[i + 1],
      color: CARDIO_COLORS[i],
    })),
  };
}

/**
 * Form as a share of fitness, in Joe Friel's zones (as intervals.icu shows them). The share
 * makes the zones independent of the size of RunningMate's load numbers.
 */
export const FORM_SCALE: Scale = {
  min: -50,
  max: 40,
  segments: [
    {
      label: tk("High risk"),
      description: tk("Fatigue is far above your fitness; too much of this risks overtraining."),
      from: -50,
      to: -30,
      color: "#e13342",
    },
    {
      label: tk("Optimal"),
      description: tk("A productive training load: your fitness is building."),
      from: -30,
      to: -10,
      color: "#701f57",
    },
    {
      label: tk("Grey zone"),
      description: tk("Maintaining: not much building, not much recovering."),
      from: -10,
      to: 5,
      color: "#a1a1aa",
    },
    {
      label: tk("Fresh"),
      description: tk("Rested and fit: a good time for a race or a test."),
      from: 5,
      to: 20,
      color: "#f37651",
    },
    {
      label: tk("Transition"),
      description: tk("Very rested: without training your fitness starts to drop."),
      from: 20,
      to: 40,
      color: "#f6b48f",
    },
  ],
};

/** `value` in percent of `reference`; `null` without a reference. */
export function shareOf(value: number, reference: number): number | null {
  return reference > 0 ? (value / reference) * 100 : null;
}

/** Change from `before` to `after` in percent; `null` without a value before. */
export function changeOf(before: number, after: number): number | null {
  return before > 0 ? ((after - before) / before) * 100 : null;
}

/** Form in percent of fitness; `null` without fitness. */
export function formPercent(form: number, fitness: number): number | null {
  return fitness > 0 ? (form / fitness) * 100 : null;
}

/** The segment a value falls in; values beyond the ends count for the outer segments. */
export function segmentOf(scale: Scale, value: number): Segment {
  return scale.segments.find((s) => value < s.to) ?? scale.segments[scale.segments.length - 1];
}

/** Position of a value on the scale, 0–100 %. */
export function position(scale: Scale, value: number): number {
  const share = (value - scale.min) / (scale.max - scale.min);
  return Math.min(100, Math.max(0, share * 100));
}
