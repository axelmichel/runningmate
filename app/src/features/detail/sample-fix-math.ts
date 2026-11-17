import type { FixSample } from "@/bindings/FixSample";
import type { SampleChannel } from "@/bindings/SampleChannel";
import type { SampleEdit } from "@/bindings/SampleEdit";
import type { SegmentRow } from "@/bindings/SegmentRow";

import { cumulativeMetres } from "./fix-math";

export type { SampleChannel, SampleEdit };

export type SampleIssueKind = "spike" | "dropout" | "startArtifact" | "cadenceLock";

/** A stretch of implausible values, as sample indices (both included). */
export type SampleIssue = { kind: SampleIssueKind; start: number; end: number };

/** A jump of more than this (bpm) between samples this close (s) is suspicious. */
const HR_JUMP = 25;
const HR_JUMP_SECONDS = 5;
/** A spike that doesn't come back within 10 bpm of where it left inside this time is real. */
const HR_SPIKE_RETURN = 10;
const HR_SPIKE_SECONDS = 300;
/** A strap settling in: a sudden drop within the first minutes, staying lower afterwards. */
const START_SECONDS = 900;
const START_DROP_SECONDS = 10;
/** Heart rate this close to the steps per minute, for this long, is the strap reading steps. */
const LOCK_BPM = 4;
const LOCK_SECONDS = 30;
const LOCK_MIN_STEPS = 120;
/** Missing values for this long count as a dropout. */
const DROPOUT_SECONDS = 10;
/** Steps missing while covering this many metres are a dropout, not a stop. */
const DROPOUT_METRES = 20;
/** More steps per minute than this is a sensor fault. */
const MAX_STEPS = 260;
/** Problems closer together than this become one stretch. */
const MERGE_SECONDS = 10;

export function valueOf(sample: FixSample, channel: SampleChannel): number | null {
  const value = channel === "heartRate" ? sample.heartRate : sample.steps;
  return value != null && value > 0 ? value : null;
}

function withValue(sample: FixSample, channel: SampleChannel, value: number | null): FixSample {
  return channel === "heartRate" ? { ...sample, heartRate: value } : { ...sample, steps: value };
}

const seconds = (samples: FixSample[], a: number, b: number) =>
  (samples[b].time - samples[a].time) / 1000;

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
      let before: FixSample | undefined;
      for (let i = first - 1; i >= 0 && !before; i--) if (valueOf(out[i], channel)) before = out[i];
      const after = out.slice(last + 1).find((s) => valueOf(s, channel) !== null);
      if (!before && !after) continue;
      for (let i = first; i <= last; i++) {
        const time = out[i].time;
        const value =
          before && after
            ? valueOf(before, channel)! +
              ((valueOf(after, channel)! - valueOf(before, channel)!) * (time - before.time)) /
                (after.time - before.time || 1)
            : valueOf((before ?? after)!, channel);
        next[i] = withValue(out[i], channel, value);
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

/** Spikes, dropouts, a strap settling in at the start and heart rate locked onto the steps. */
export function detectHeartRateIssues(samples: FixSample[]): SampleIssue[] {
  if (!samples.some((s) => valueOf(s, "heartRate") !== null)) return [];
  const issues: SampleIssue[] = [];
  const hr = (i: number) => valueOf(samples[i], "heartRate");

  // Dropouts between the first and last value (none at all before the strap connects is fine).
  const firstValue = samples.findIndex((s) => valueOf(s, "heartRate") !== null);
  for (const [start, end] of runs(
    samples,
    (s, i) => i > firstValue && valueOf(s, "heartRate") === null,
    DROPOUT_SECONDS,
  )) {
    if (end < samples.length - 1) issues.push({ kind: "dropout", start, end });
  }

  // Jumps: a spike when it comes back, a strap settling in when it drops early and stays down.
  let previous: number | null = null;
  for (let i = 0; i < samples.length; i++) {
    const value = hr(i);
    if (value === null) continue;
    if (previous !== null && seconds(samples, previous, i) <= HR_JUMP_SECONDS) {
      const base = hr(previous)!;
      if (Math.abs(value - base) > HR_JUMP) {
        let back: number | null = null;
        for (let j = i + 1; j < samples.length && seconds(samples, i, j) <= HR_SPIKE_SECONDS; j++) {
          const v = hr(j);
          if (v !== null && Math.abs(v - base) <= HR_SPIKE_RETURN) {
            back = j;
            break;
          }
        }
        if (back !== null) {
          issues.push({ kind: "spike", start: i, end: back - 1 });
          previous = back;
          i = back;
          continue;
        }
        if (value < base && seconds(samples, 0, i) <= START_SECONDS) {
          issues.push({ kind: "startArtifact", start: 0, end: previous });
        }
      }
    }
    previous = i;
  }

  // A sudden fall over a few samples early on also counts as settling in.
  if (!issues.some((issue) => issue.kind === "startArtifact")) {
    for (let i = 0; i < samples.length && seconds(samples, 0, i) <= START_SECONDS; i++) {
      const value = hr(i);
      // The way down from a spike isn't the strap settling in.
      if (value === null || issues.some((x) => i >= x.start && i <= x.end + 1)) continue;
      for (let j = i + 1; j < samples.length && seconds(samples, i, j) <= START_DROP_SECONDS; j++) {
        const later = hr(j);
        if (later !== null && value - later > HR_JUMP) {
          const stays = samples
            .slice(j, j + 60)
            .every((s) => (valueOf(s, "heartRate") ?? later) < value - HR_JUMP / 2);
          if (stays) {
            issues.push({ kind: "startArtifact", start: 0, end: j - 1 });
            i = samples.length;
          }
          break;
        }
      }
    }
  }

  for (const [start, end] of runs(
    samples,
    (s) => {
      const value = valueOf(s, "heartRate");
      const steps = valueOf(s, "steps");
      return (
        value !== null &&
        steps !== null &&
        steps >= LOCK_MIN_STEPS &&
        Math.abs(value - steps) <= LOCK_BPM
      );
    },
    LOCK_SECONDS,
  )) {
    issues.push({ kind: "cadenceLock", start, end });
  }
  return merged(samples, issues);
}

/** Steps missing while moving, and impossible step rates. */
export function detectStepsIssues(samples: FixSample[]): SampleIssue[] {
  if (!samples.some((s) => valueOf(s, "steps") !== null)) return [];
  const metres = cumulativeMetres(samples);
  const issues: SampleIssue[] = [];
  for (const [start, end] of runs(samples, (s) => valueOf(s, "steps") === null, DROPOUT_SECONDS)) {
    if (metres[end] - metres[start] >= DROPOUT_METRES) issues.push({ kind: "dropout", start, end });
  }
  for (const [start, end] of runs(samples, (s) => (s.steps ?? 0) > MAX_STEPS, 0)) {
    issues.push({ kind: "spike", start, end });
  }
  return merged(samples, issues);
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
