import { ArrowDownUp, Equal, Spline, Trash2, X } from "lucide-react";
import { useMemo, useState } from "react";
import { CartesianGrid, Line, LineChart, ReferenceArea, XAxis, YAxis } from "recharts";
import type { CategoricalChartFunc } from "recharts/types/chart/types";

import type { FixSource } from "@/bindings/FixSource";
import type { SampleOp } from "@/bindings/SampleOp";
import { Button } from "@/components/ui/button";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { Input } from "@/components/ui/input";
import { t, tk } from "@/i18n";
import { formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { cumulativeMetres } from "./fix-math";
import {
  detectHeartRateIssues,
  detectStepsIssues,
  previewSampleEdits,
  valueOf,
  type SampleChannel,
  type SampleEdit,
  type SampleIssue,
  type SampleIssueKind,
} from "./sample-fix-math";
import { useUnsavedChanges } from "./use-unsaved-guard";

/** The chart draws at most this many points; selections still resolve to single samples. */
const MAX_POINTS = 2000;

const ISSUE_LABELS: Record<SampleIssueKind, string> = {
  spike: tk("Spike"),
  dropout: tk("Dropout"),
  startArtifact: tk("Implausible start"),
  cadenceLock: tk("Reads the steps"),
};

const CHANNELS: Record<SampleChannel, { label: string; unit: string; color: string }> = {
  heartRate: { label: tk("Heart rate (bpm)"), unit: "bpm", color: "var(--chart-3)" },
  steps: { label: tk("Steps per minute"), unit: "spm", color: "var(--chart-5)" },
};

type Point = { i: number; km: number; original: number | null; edited: number | null };

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {title}
      </h3>
      {children}
    </section>
  );
}

function average(values: (number | null)[]): number | null {
  const valid = values.filter((v): v is number => v !== null);
  return valid.length ? valid.reduce((a, b) => a + b, 0) / valid.length : null;
}

/**
 * Corrects the heart rate or steps of stretches: drag over the chart (or pick a detected
 * problem) to select one, then interpolate it, set it to a value or shift it. `onSave` gets
 * all corrections; the backend applies them to the track and recomputes the activity.
 */
export function SampleFixEditor({
  source,
  channel,
  saving,
  onSave,
  extraActions,
}: {
  source: FixSource;
  channel: SampleChannel;
  saving: boolean;
  /** Saves the corrections; a promise that fails when they couldn't be saved. */
  onSave: (edits: SampleEdit[]) => Promise<unknown>;
  /** Further buttons below Save, such as resetting to the original. */
  extraActions?: React.ReactNode;
}) {
  const { samples } = source;
  const { label, unit, color } = CHANNELS[channel];
  const [edits, setEdits] = useState<SampleEdit[]>([]);
  /** Selected samples, both included. */
  const [range, setRange] = useState<[number, number] | null>(null);
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [setValue, setSetValue] = useState("");
  const [offset, setOffset] = useState("");

  useUnsavedChanges(edits.length > 0, () => onSave(edits));

  const km = useMemo(() => cumulativeMetres(samples).map((m) => m / 1000), [samples]);
  const preview = useMemo(() => previewSampleEdits(samples, edits), [samples, edits]);
  const issues = useMemo(
    () => (channel === "heartRate" ? detectHeartRateIssues(samples) : detectStepsIssues(samples)),
    [samples, channel],
  );
  const data = useMemo<Point[]>(() => {
    const stride = Math.max(1, Math.ceil(samples.length / MAX_POINTS));
    const points: Point[] = [];
    for (let i = 0; i < samples.length; i += stride) {
      points.push({
        i,
        km: km[i],
        original: valueOf(samples[i], channel),
        edited: valueOf(preview[i], channel),
      });
    }
    return points;
  }, [samples, preview, km, channel]);
  const hasValues = samples.some((s) => valueOf(s, channel) !== null);

  const sampleAt = (state: Parameters<CategoricalChartFunc>[0]) => {
    const index = Number(state.activeTooltipIndex);
    return Number.isInteger(index) && data[index] ? data[index].i : null;
  };
  const sorted = (a: number, b: number): [number, number] => (a <= b ? [a, b] : [b, a]);
  const select = (issue: SampleIssue) => setRange([issue.start, issue.end]);
  const addEdit = (op: SampleOp) => {
    if (!range) return;
    setEdits([...edits, { channel, from: samples[range[0]].time, to: samples[range[1]].time, op }]);
    setRange(null);
  };
  const nearest = (value: number) => {
    let best = 0;
    km.forEach((k, i) => {
      if (Math.abs(k - value) < Math.abs(km[best] - value)) best = i;
    });
    return best;
  };
  const kmOf = (time: number) =>
    km[
      Math.max(
        0,
        samples.findIndex((s) => s.time >= time),
      )
    ] ?? 0;
  const opLabel = (op: SampleOp) =>
    op.kind === "interpolate"
      ? t("Interpolated")
      : op.kind === "set"
        ? t("Set to {{value}}", { value: `${formatNumber(op.value)} ${unit}` })
        : t("Shifted by {{value}}", {
            value: `${op.value > 0 ? "+" : ""}${formatNumber(op.value)} ${unit}`,
          });
  const before = average(samples.map((s) => valueOf(s, channel)));
  const after = average(preview.map((s) => valueOf(s, channel)));
  const selectionAverage = range
    ? average(preview.slice(range[0], range[1] + 1).map((s) => valueOf(s, channel)))
    : null;
  const config = { edited: { label: t(label), color }, original: { label: t("Recorded") } };

  return (
    <div className="flex h-full min-h-0 gap-4">
      <div className="flex min-h-72 min-w-0 flex-1 flex-col gap-2 select-none">
        <ChartContainer
          config={config satisfies ChartConfig}
          className="aspect-auto min-h-0 w-full flex-1 cursor-crosshair"
        >
          <LineChart
            data={data}
            margin={{ top: 8, right: 12, bottom: 0, left: 0 }}
            onMouseDown={(state) => {
              const i = sampleAt(state);
              if (i === null) return;
              setDragFrom(i);
              setRange([i, i]);
            }}
            onMouseMove={(state) => {
              const i = sampleAt(state);
              if (dragFrom !== null && i !== null) setRange(sorted(dragFrom, i));
            }}
            onMouseUp={() => {
              setDragFrom(null);
              if (range && range[0] === range[1]) setRange(null);
            }}
            onMouseLeave={() => setDragFrom(null)}
          >
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="km"
              type="number"
              domain={["dataMin", "dataMax"]}
              tickFormatter={(v: number) => `${formatNumber(v, 1)} km`}
              tickLine={false}
              axisLine={false}
            />
            <YAxis
              width={44}
              tickLine={false}
              axisLine={false}
              domain={["dataMin - 5", "dataMax + 5"]}
              tickFormatter={(v: number) => formatNumber(v)}
            />
            <ChartTooltip
              content={
                <ChartTooltipContent
                  labelFormatter={(_, payload) =>
                    `${formatNumber((payload?.[0]?.payload as Point | undefined)?.km, 2)} km`
                  }
                />
              }
            />
            {range && (
              <ReferenceArea
                x1={km[range[0]]}
                x2={km[range[1]]}
                fill="var(--primary)"
                fillOpacity={0.12}
                stroke="var(--primary)"
                strokeOpacity={0.4}
              />
            )}
            {edits.length > 0 && (
              <Line
                dataKey="original"
                type="monotone"
                stroke="var(--muted-foreground)"
                strokeOpacity={0.5}
                strokeDasharray="3 3"
                dot={false}
                strokeWidth={1}
                connectNulls
                isAnimationActive={false}
              />
            )}
            <Line
              dataKey="edited"
              type="monotone"
              stroke="var(--color-edited)"
              dot={false}
              strokeWidth={1.5}
              connectNulls
              isAnimationActive={false}
            />
          </LineChart>
        </ChartContainer>
        <p className="text-xs text-muted-foreground">
          {hasValues
            ? t("Drag over the chart to select a stretch.")
            : t("This activity has no values recorded here.")}
        </p>
      </div>

      <aside className="flex w-72 shrink-0 flex-col gap-5 overflow-y-auto">
        <Section title={t("Problems found")}>
          {issues.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("No problems found.")}</p>
          ) : (
            <div className="flex flex-col">
              {issues.map((issue, i) => (
                <button
                  key={i}
                  type="button"
                  className={cn(
                    "flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors hover:bg-muted",
                    range?.[0] === issue.start && range[1] === issue.end && "bg-muted",
                  )}
                  onClick={() => select(issue)}
                >
                  <span className="flex items-center gap-2">
                    <span className="size-2 rounded-full bg-destructive" />
                    {t(ISSUE_LABELS[issue.kind])}
                  </span>
                  <span className="text-xs text-muted-foreground tabular-nums">
                    {formatNumber(km[issue.start], 2)} – {formatNumber(km[issue.end], 2)} km
                  </span>
                </button>
              ))}
            </div>
          )}
        </Section>

        {range && (
          <Section title={t("Selection")}>
            <div className="flex items-center gap-2 text-sm tabular-nums">
              <Input
                type="number"
                step="0.01"
                min={0}
                aria-label={t("From (km)")}
                className="h-8"
                // Applied on leaving the field, so typing isn't snapped to a sample midway.
                key={`0-${range[0]}`}
                defaultValue={km[range[0]].toFixed(2)}
                onBlur={(e) => {
                  const i = nearest(Number(e.target.value));
                  setRange(sorted(i, range[1]));
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                }}
              />
              –
              <Input
                type="number"
                step="0.01"
                min={0}
                aria-label={t("To (km)")}
                className="h-8"
                key={`1-${range[1]}`}
                defaultValue={km[range[1]].toFixed(2)}
                onBlur={(e) => {
                  const i = nearest(Number(e.target.value));
                  setRange(sorted(i, range[0]));
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") e.currentTarget.blur();
                }}
              />
              km
            </div>
            {selectionAverage !== null && (
              <p className="text-xs text-muted-foreground tabular-nums">
                {t("Average {{value}}", { value: `${formatNumber(selectionAverage)} ${unit}` })}
              </p>
            )}
            <Button size="sm" variant="outline" onClick={() => addEdit({ kind: "interpolate" })}>
              <Spline />
              {t("Interpolate")}
            </Button>
            <div className="flex gap-2">
              <Input
                type="number"
                min={0}
                placeholder={unit}
                aria-label={t("Value")}
                className="h-8"
                value={setValue}
                onChange={(e) => setSetValue(e.target.value)}
              />
              <Button
                size="sm"
                variant="outline"
                disabled={!(Number(setValue) > 0)}
                onClick={() => addEdit({ kind: "set", value: Number(setValue) })}
              >
                <Equal />
                {t("Set")}
              </Button>
            </div>
            <div className="flex gap-2">
              <Input
                type="number"
                placeholder={`± ${unit}`}
                aria-label={t("Offset")}
                className="h-8"
                value={offset}
                onChange={(e) => setOffset(e.target.value)}
              />
              <Button
                size="sm"
                variant="outline"
                disabled={!Number(offset)}
                onClick={() => addEdit({ kind: "offset", value: Number(offset) })}
              >
                <ArrowDownUp />
                {t("Shift")}
              </Button>
            </div>
            <Button size="sm" variant="ghost" onClick={() => setRange(null)}>
              <X />
              {t("Cancel")}
            </Button>
          </Section>
        )}

        <Section title={t("Fixes")}>
          {edits.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("No fixes yet.")}</p>
          ) : (
            <ul className="flex flex-col gap-1">
              {edits.map((edit, i) => (
                <li key={i} className="flex items-center justify-between gap-2 text-sm">
                  <span className="grid">
                    <span className="tabular-nums">
                      {formatNumber(kmOf(edit.from), 2)} – {formatNumber(kmOf(edit.to), 2)} km
                    </span>
                    <span className="text-xs text-muted-foreground">{opLabel(edit.op)}</span>
                  </span>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="size-7"
                    aria-label={t("Remove")}
                    onClick={() => setEdits(edits.filter((_, j) => j !== i))}
                  >
                    <Trash2 />
                  </Button>
                </li>
              ))}
            </ul>
          )}
          <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-sm tabular-nums">
            <dt className="text-muted-foreground">{t("Recorded")}</dt>
            <dd className="text-right">
              {before === null ? "–" : `${formatNumber(before)} ${unit}`}
            </dd>
            <dt className="text-muted-foreground">{t("After the fixes")}</dt>
            <dd className="text-right font-semibold">
              {after === null ? "–" : `${formatNumber(after)} ${unit}`}
            </dd>
          </dl>
          <Button
            onClick={() => void onSave(edits).catch(() => {})}
            disabled={edits.length === 0 || saving}
          >
            {saving ? t("Saving…") : t("Save and recompute")}
          </Button>
          {extraActions}
        </Section>
      </aside>
    </div>
  );
}
