import type { FixSample } from "@/bindings/FixSample";
import type { SampleChannel } from "@/bindings/SampleChannel";
import type { SampleEdit } from "@/bindings/SampleEdit";
import type { SegmentRow } from "@/bindings/SegmentRow";

import { cumulativeMetres } from "./fix-math";

export type { SampleChannel, SampleEdit };

export type SampleIssueKind = "missing" | "dropout" | "outlier" | "startArtifact";

/** A stretch of implausible or missing values, as sample indices (both included). */
export type SampleIssue = { kind: SampleIssueKind; start: number; end: number };

/** Values are compared with the median of the samples this many seconds around them. */
const BASELINE_SECONDS = 30;
/** Further than this from that median (bpm, steps per minute) is an outlier. */
const OUTLIER: Record<SampleChannel, number> = { heartRate: 20, steps: 25 };
/** A strap settling in: a drop of more than this (bpm) within the first minutes… */
const START_DROP = 25;
const START_SECONDS = 900;
const START_DROP_SECONDS = 10;
/** …after which the heart rate stays this much lower for a minute. */
const START_STAYS = 12;
/** Missing values for this long count as a dropout. */
const DROPOUT_SECONDS = 10;
/** Steps missing while covering this many metres are a dropout, not a stop. */
const DROPOUT_METRES = 20;
/** Problems closer together than this become one stretch. */
const MERGE_SECONDS = 10;
/** Interpolation leans on the median of the values this many seconds outside a stretch. */
export const ANCHOR_SECONDS = 30;

export function valueOf(sample: FixSample, channel: SampleChannel): number | null {
  const value = channel === "heartRate" ? sample.heartRate : sample.steps;
  return value != null && value > 0 ? value : null;
}

function withValue(sample: FixSample, channel: SampleChannel, value: number | null): FixSample {
  return channel === "heartRate" ? { ...sample, heartRate: value } : { ...sample, steps: value };
}

const seconds = (samples: FixSample[], a: number, b: number) =>
  (samples[b].time - samples[a].time) / 1000;

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = sorted.length >> 1;
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/**
 * What an interpolation starts or ends at, beside the stretch from `edge` going `step` (−1
 * before, +1 after): the median of the values within [`ANCHOR_SECONDS`] of the nearest one, so
 * a single stray value next to the stretch doesn't set the level. The backend does the same.
 */
function anchor(
  samples: FixSample[],
  channel: SampleChannel,
  edge: number,
  step: 1 | -1,
): { time: number; value: number } | null {
  let i = edge + step;
  while (i >= 0 && i < samples.length && valueOf(samples[i], channel) === null) i += step;
  if (i < 0 || i >= samples.length) return null;
  const time = samples[i].time;
  const values: number[] = [];
  for (let j = i; j >= 0 && j < samples.length; j += step) {
    if (Math.abs(samples[j].time - time) > ANCHOR_SECONDS * 1000) break;
    const value = valueOf(samples[j], channel);
    if (value !== null) values.push(value);
  }
  return { time, value: median(values) };
}

/** The samples with the edits applied, as the backend applies them (for the preview). */
export function previewSampleEdits(samples: FixSample[], edits: SampleEdit[]): FixSample[] {
  let out = samples;
  for (const edit of edits) {
    const inside = (s: FixSample) => s.time >= edit.from && s.time <= edit.to;
    const first = out.findIndex(inside);
    if (first < 0) continue;
    let last = first;
    while (last + 1 < out.length && inside(out[last + 1])) last += 1;
    const { channel, op } = edit;
    const next = [...out];
    if (op.kind === "interpolate") {
      const before = anchor(out, channel, first, -1);
      const after = anchor(out, channel, last, 1);
      if (!before && !after) continue;
      for (let i = first; i <= last; i++) {
        const time = out[i].time;
        const value =
          before && after
            ? before.value +
              ((after.value - before.value) * (time - before.time)) /
                (after.time - before.time || 1)
            : (before ?? after)!.value;
        next[i] = withValue(out[i], channel, value);
      }
    } else if (op.kind === "values") {
      if (op.values.length !== last - first + 1) continue;
      for (let i = first; i <= last; i++) {
        const value = op.values[i - first];
        next[i] = withValue(out[i], channel, value > 0 ? value : null);
      }
    } else {
      for (let i = first; i <= last; i++) {
        const current = channel === "heartRate" ? out[i].heartRate : out[i].steps;
        const value =
          op.kind === "set"
            ? op.value > 0
              ? op.value
              : null
            : current == null
              ? null
              : Math.max(0, current + op.value);
        next[i] = withValue(out[i], channel, value);
      }
    }
    out = next;
  }
  return out;
}

/** Joins issues that overlap or lie close together, in time order. */
function merged(samples: FixSample[], issues: SampleIssue[]): SampleIssue[] {
  const sorted = [...issues].sort((a, b) => a.start - b.start);
  const out: SampleIssue[] = [];
  for (const issue of sorted) {
    const previous = out.at(-1);
    if (previous && seconds(samples, previous.end, issue.start) <= MERGE_SECONDS) {
      previous.end = Math.max(previous.end, issue.end);
    } else {
      out.push({ ...issue });
    }
  }
  return out;
}

/** Runs of samples matching `test` lasting at least `minSeconds`. */
function runs(
  samples: FixSample[],
  test: (sample: FixSample, index: number) => boolean,
  minSeconds: number,
): [number, number][] {
  const out: [number, number][] = [];
  let start: number | null = null;
  samples.forEach((sample, i) => {
    const matches = test(sample, i);
    if (matches && start === null) start = i;
    const closes = start !== null && (!matches || i === samples.length - 1);
    if (closes) {
      const end = matches ? i : i - 1;
      if (seconds(samples, start!, end) >= minSeconds) out.push([start!, end]);
      start = null;
    }
  });
  return out;
}

/** Medians of the values in the [`BASELINE_SECONDS`] before and after each sample. */
function sideLevels(
  samples: FixSample[],
  channel: SampleChannel,
): { before: number | null; after: number | null }[] {
  const window = BASELINE_SECONDS * 1000;
  const level = (from: number, to: number) => {
    const values = samples
      .slice(from, to)
      .map((s) => valueOf(s, channel))
      .filter((v): v is number => v !== null);
    return values.length ? median(values) : null;
  };
  let from = 0;
  let to = 0;
  return samples.map((sample, i) => {
    while (samples[from].time < sample.time - window) from += 1;
    while (to < samples.length && samples[to].time <= sample.time + window) to += 1;
    return { before: level(from, i), after: level(i + 1, to) };
  });
}

/**
 * Missing values (at the start or end, or a dropout in between), values far off the run's
 * level around them, and for heart rate a strap settling in: reading high in the first minutes,
 * then dropping suddenly.
 */
function detect(samples: FixSample[], channel: SampleChannel): SampleIssue[] {
  const has = (i: number) => valueOf(samples[i], channel) !== null;
  const first = samples.findIndex((_, i) => has(i));
  if (first < 0) return [];
  let last = samples.length - 1;
  while (!has(last)) last -= 1;
  const issues: SampleIssue[] = [];
  if (seconds(samples, 0, first) >= DROPOUT_SECONDS) {
    issues.push({ kind: "missing", start: 0, end: first - 1 });
  }
  if (seconds(samples, last, samples.length - 1) >= DROPOUT_SECONDS) {
    issues.push({ kind: "missing", start: last + 1, end: samples.length - 1 });
  }
  // Steps stop when you stand still; only missing steps while moving are a dropout.
  const metres = channel === "steps" ? cumulativeMetres(samples) : null;
  for (const [start, end] of runs(
    samples,
    (_, i) => i > first && i < last && !has(i),
    DROPOUT_SECONDS,
  )) {
    if (!metres || metres[end] - metres[start] >= DROPOUT_METRES) {
      issues.push({ kind: "dropout", start, end });
    }
  }

  // Beyond the levels on both sides: a spike. Between them, or close to one, is a change of
  // level (a hill, a walk break, slowing down) and fine. Early on only what follows counts,
  // since the start itself may be the problem.
  const levels = sideLevels(samples, channel);
  const limit = OUTLIER[channel];
  const off = (s: FixSample, i: number) => {
    const value = valueOf(s, channel);
    if (value === null) return false;
    const early = seconds(samples, first, i) < 2 * BASELINE_SECONDS;
    const sides = [early ? null : levels[i].before, levels[i].after].filter(
      (v): v is number => v !== null,
    );
    // Starting low and warming up is normal; at the start only reading too high is wrong.
    return (
      sides.length > 0 &&
      (value > Math.max(...sides) + limit || (!early && value < Math.min(...sides) - limit))
    );
  };
  for (const [start, end] of runs(samples, off, 0)) {
    if (!(metres && slowedDown(samples, metres, levels, channel, start, end))) {
      issues.push({ kind: "outlier", start, end });
    }
  }

  if (channel === "heartRate") {
    const settled = startArtifact(samples);
    if (settled !== null) issues.push({ kind: "startArtifact", start: 0, end: settled });
  }
  return merged(samples, issues);
}

/** Below this share of the speed around it, fewer steps are real: you slowed down or stopped. */
const SLOWER = 0.85;

/** Whether a stretch of too few steps comes with lower speed, so the steps are right. */
function slowedDown(
  samples: FixSample[],
  metres: number[],
  levels: { before: number | null; after: number | null }[],
  channel: SampleChannel,
  start: number,
  end: number,
): boolean {
  const value = valueOf(samples[start], channel)!;
  const { before, after } = levels[start];
  if (value > Math.min(...[before, after].filter((v): v is number => v !== null))) return false;
  const speed = (a: number, b: number) =>
    b > a ? (metres[b] - metres[a]) / Math.max(1, seconds(samples, a, b)) : null;
  const window = BASELINE_SECONDS * 1000;
  let from = start;
  while (from > 0 && samples[from - 1].time >= samples[start].time - window) from -= 1;
  let to = end;
  while (to < samples.length - 1 && samples[to + 1].time <= samples[end].time + window) to += 1;
  const around = [speed(from, start), speed(end, to)].filter((v): v is number => v !== null);
  const inside = speed(Math.max(0, start - 1), end) ?? 0;
  return around.length === 0 || inside < SLOWER * Math.max(...around);
}

/** Last sample of a strap reading high before a sudden drop early on, or `null`. */
function startArtifact(samples: FixSample[]): number | null {
  const hr = (i: number) => valueOf(samples[i], "heartRate");
  for (let i = 0; i < samples.length && seconds(samples, 0, i) <= START_SECONDS; i++) {
    const value = hr(i);
    if (value === null) continue;
    for (let j = i + 1; j < samples.length && seconds(samples, i, j) <= START_DROP_SECONDS; j++) {
      const later = hr(j);
      if (later === null || value - later <= START_DROP) continue;
      // Real only when it stays down for the next minute.
      let k = j;
      const after: number[] = [];
      while (k < samples.length && seconds(samples, j, k) <= 60) {
        const v = hr(k++);
        if (v !== null) after.push(v);
      }
      const before = samples
        .slice(0, i + 1)
        .map((s) => valueOf(s, "heartRate"))
        .filter((v): v is number => v !== null);
      // A strap reads high from the first seconds; warming up starts low.
      const opening = samples
        .filter((s) => s.time - samples[0].time <= BASELINE_SECONDS * 1000)
        .map((s) => valueOf(s, "heartRate"))
        .filter((v): v is number => v !== null);
      const rest = samples
        .slice(j)
        .map((s) => valueOf(s, "heartRate"))
        .filter((v): v is number => v !== null);
      const high = opening.length > 0 && median(opening) > median(rest) + START_STAYS;
      if (high && after.length && median(after) < median(before) - START_STAYS) return j - 1;
      break;
    }
  }
  return null;
}

/** Problems of the heart rate. */
export function detectHeartRateIssues(samples: FixSample[]): SampleIssue[] {
  return detect(samples, "heartRate");
}

/** Problems of the steps. */
export function detectStepsIssues(samples: FixSample[]): SampleIssue[] {
  return detect(samples, "steps");
}

/** Speed is averaged over this many seconds up to a sample: heart rate follows effort late. */
const EFFORT_SECONDS = 60;
/** How quickly heart rate (and, much quicker, the steps) follow a change of effort (s). */
const RESPONSE_SECONDS: Record<SampleChannel, number> = { heartRate: 45, steps: 3 };
/** Recorded samples needed to learn how the values follow the speed. */
const MIN_FIT_SAMPLES = 120;

/** m/s averaged over the [`EFFORT_SECONDS`] up to each sample. */
function effortSpeeds(samples: FixSample[]): number[] {
  const metres = cumulativeMetres(samples);
  let from = 0;
  return samples.map((sample, i) => {
    while (samples[from].time < sample.time - EFFORT_SECONDS * 1000) from += 1;
    const seconds = (sample.time - samples[from].time) / 1000;
    return seconds > 0 ? (metres[i] - metres[from]) / seconds : 0;
  });
}

/**
 * Values for samples `first` to `last` estimated from the speed, for a stretch where they are
 * missing or wrong: how the values follow the speed is learned from the rest of the activity
 * (a straight line through value against speed), the heart rate climbs towards that level
 * with a delay as it does at the start of a run, and the estimate is bent to meet the
 * recorded values on both sides. `null` when the rest of the activity doesn't tell enough.
 */
export function estimateStretch(
  samples: FixSample[],
  channel: SampleChannel,
  first: number,
  last: number,
): number[] | null {
  const speeds = effortSpeeds(samples);
  // Value against speed on the recorded samples outside the stretch, while moving.
  const points: [number, number][] = [];
  samples.forEach((sample, i) => {
    const value = valueOf(sample, channel);
    if ((i < first || i > last) && value !== null && speeds[i] > 0.5) {
      points.push([speeds[i], value]);
    }
  });
  if (points.length < MIN_FIT_SAMPLES) return null;
  const n = points.length;
  const meanV = points.reduce((sum, [v]) => sum + v, 0) / n;
  const meanY = points.reduce((sum, [, y]) => sum + y, 0) / n;
  const varV = points.reduce((sum, [v]) => sum + (v - meanV) ** 2, 0);
  const slope =
    varV > 0 ? points.reduce((sum, [v, y]) => sum + (v - meanV) * (y - meanY), 0) / varV : 0;
  // Faster never means a lower heart rate or fewer steps; without a relation the level stays.
  const b = Math.max(0, slope);
  const level = (speed: number) => meanY + b * (speed - meanV);

  const before = anchor(samples, channel, first, -1);
  const after = anchor(samples, channel, last, 1);
  // At the very start the heart rate is still low, about where standing still would put it,
  // but no lower than 55 % and no higher than 75 % of the level the recording settles at.
  const settled = after?.value ?? meanY;
  const start =
    before?.value ??
    (channel === "heartRate"
      ? Math.min(0.75 * settled, Math.max(0.55 * settled, level(0)))
      : level(speeds[first]));
  const tau = RESPONSE_SECONDS[channel];
  const estimate: number[] = [];
  let value = start;
  let time = before?.time ?? samples[first].time;
  for (let i = first; i <= last; i++) {
    const dt = Math.max(0, (samples[i].time - time) / 1000);
    value += (level(speeds[i]) - value) * (1 - Math.exp(-dt / tau));
    time = samples[i].time;
    estimate.push(value);
  }
  // Bent to end where the recording goes on, growing from nothing at the start of the stretch.
  if (after) {
    const t0 = samples[first].time;
    const span = samples[last].time - t0 || 1;
    const miss = after.value - estimate[estimate.length - 1];
    for (let i = first; i <= last; i++) {
      estimate[i - first] += (miss * (samples[i].time - t0)) / span;
    }
  }
  return estimate.map((v) => Math.max(1, Math.round(v)));
}

/** A changed split: the average heart rate and steps per minute it should have. */
export type SplitChange = { heartRate?: number; steps?: number };

/**
 * Shifts the samples of each changed split so its average becomes the entered value. Splits share
 * their boundary sample with the next one, so a split ends just before the next starts.
 */
export function splitEdits(
  segments: SegmentRow[],
  changes: Record<number, SplitChange>,
): SampleEdit[] {
  const edits: SampleEdit[] = [];
  segments.forEach((segment, i) => {
    const change = changes[segment.index];
    if (!change || segment.start == null || segment.end == null) return;
    const to = i < segments.length - 1 ? segment.end - 1 : segment.end;
    const shift = (channel: SampleChannel, target: number | undefined, current: number | null) => {
      if (target === undefined || current == null || target === current) return;
      edits.push({
        channel,
        from: segment.start!,
        to,
        op: { kind: "offset", value: target - current },
      });
    };
    shift("heartRate", change.heartRate, segment.heartRate);
    shift("steps", change.steps, segment.steps == null ? null : segment.steps * 2);
  });
  return edits;
}
