import { useMemo, useState } from "react";
import { CartesianGrid, Line, LineChart, ReferenceLine, XAxis, YAxis } from "recharts";

import type { TrackPoint } from "@/bindings/TrackPoint";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { robustRange } from "@/features/detail/color-scale";
import { speedFromPace } from "@/features/detail/track-position";
import { t, tk } from "@/i18n";
import { formatDuration, formatNumber, formatPace } from "@/lib/format";

import {
  onFoot,
  overlayRows,
  raceGaps,
  sliceStretch,
  speedMode,
  splitRows,
  splitsComparable,
  type Series,
} from "./compare-math";
import { EntryLegend } from "./compare-figures";
import type { CompareEntry } from "./use-compare-data";

type Axis = "distance" | "elapsed";

const AXES: { axis: Axis; label: string }[] = [
  { axis: "distance", label: tk("Distance") },
  { axis: "elapsed", label: tk("Time") },
];

/** "1:05" for seconds, without the hours while under one. */
const formatClock = (seconds: number) => formatDuration(seconds).replace(/^0:/, "");

/** Seconds behind (+) or ahead (−) of the fastest. */
function formatGap(seconds: number): string {
  const rounded = Math.round(seconds);
  if (rounded === 0) return "0:00";
  return `${rounded > 0 ? "+" : "−"}${formatClock(Math.abs(rounded))}`;
}

const formatKm = (km: number, digits = 2) => `${formatNumber(km, digits)} km`;

function config(entries: CompareEntry[]): ChartConfig {
  return Object.fromEntries(entries.map((e) => [e.key, { label: e.label, color: e.color }]));
}

function ChartCard({
  title,
  note,
  action,
  children,
}: {
  title: string;
  note?: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    // Not clipped: the tooltip may reach beyond the card.
    <Card size="sm" className="overflow-visible">
      <CardHeader className="flex flex-row flex-wrap items-center justify-between gap-2">
        <CardTitle className="text-sm">{title}</CardTitle>
        {action}
      </CardHeader>
      <CardContent className="grid min-w-0 gap-2">
        {note && <p className="text-xs text-muted-foreground">{note}</p>}
        {children}
      </CardContent>
    </Card>
  );
}

/** One line per activity over a shared x axis, with a tooltip listing all of them. */
function LinesChart({
  rows,
  entries,
  xKey,
  formatX,
  formatY,
  formatTick,
  robust,
  reversed = false,
  zeroLine = false,
  dots = false,
  category = false,
  onOpen,
}: {
  rows: Record<string, number>[];
  entries: CompareEntry[];
  xKey: string;
  formatX: (value: number) => string;
  formatY: (value: number) => string;
  /** Axis labels, short and without unit (the unit is in the title and the tooltip). */
  formatTick?: (value: number) => string;
  /** Scale to the bulk of the values, so a few stops don't flatten the curves. */
  robust?: boolean;
  /** Faster (lower) paces at the top. */
  reversed?: boolean;
  zeroLine?: boolean;
  dots?: boolean;
  /** The x values are labels (split numbers), not a continuous scale. */
  category?: boolean;
  onOpen: (id: number) => void;
}) {
  const shown = entries.filter((e) => rows.some((row) => row[e.key] != null));
  const domain = useMemo((): [number | string, number | string] => {
    if (!robust) return ["auto", "auto"];
    const values = rows.flatMap((row) =>
      shown.map((e) => row[e.key]).filter((v): v is number => typeof v === "number"),
    );
    const [low, high] = robustRange(values);
    const pad = (high - low) * 0.15;
    return [low - pad, high + pad];
  }, [rows, shown, robust]);
  return (
    <>
      <ChartContainer config={config(shown)} className="aspect-auto h-56 w-full">
        <LineChart data={rows} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey={xKey}
            type={category ? "category" : "number"}
            domain={category ? undefined : ["dataMin", "dataMax"]}
            tickFormatter={(v: number) => formatX(v)}
            tickLine={false}
            axisLine={false}
            minTickGap={24}
          />
          <YAxis
            width={52}
            reversed={reversed}
            tickLine={false}
            axisLine={false}
            domain={domain}
            allowDataOverflow={robust}
            tickFormatter={(v: number) => (formatTick ?? formatY)(v)}
          />
          {zeroLine && <ReferenceLine y={0} stroke="var(--muted-foreground)" />}
          <ChartTooltip
            content={
              <ChartTooltipContent
                labelFormatter={(_, payload) =>
                  formatX(Number((payload?.[0]?.payload as Record<string, number>)?.[xKey]))
                }
                formatter={(value, name, item) => (
                  <div className="flex flex-1 items-center justify-between gap-4 leading-none">
                    <span className="flex items-center gap-1.5 text-muted-foreground">
                      <span
                        aria-hidden
                        className="size-2.5 rounded-[2px]"
                        style={{ backgroundColor: item.color }}
                      />
                      {shown.find((e) => e.key === name)?.label}
                    </span>
                    <span className="font-mono font-medium text-foreground tabular-nums">
                      {formatY(Number(value))}
                    </span>
                  </div>
                )}
              />
            }
          />
          {shown.map((entry) => (
            <Line
              key={entry.key}
              dataKey={entry.key}
              type="monotone"
              stroke={`var(--color-${entry.key})`}
              strokeWidth={2}
              dot={dots ? { r: 3, fill: `var(--color-${entry.key})` } : false}
              connectNulls
              isAnimationActive={false}
            />
          ))}
        </LineChart>
      </ChartContainer>
      <EntryLegend entries={shown} onOpen={onOpen} />
    </>
  );
}

/** Each activity's points, cut to its segment pass when comparing the stretches. */
function seriesOf(entries: CompareEntry[], stretched: boolean): Series[] {
  return entries
    .filter((e) => e.series?.length)
    .map((e) => ({
      key: e.key,
      points: sliceStretch(e.series ?? [], stretched ? e.stretch : null),
    }));
}

type Metric = {
  title: string;
  value: (p: TrackPoint) => number | null;
  format: (v: number) => string;
  tick: (v: number) => string;
  robust?: boolean;
  reversed?: boolean;
  /** Only runs and walks have it. */
  onFoot?: boolean;
};

/**
 * Pace (or speed, see `speedMode`), GAP, heart rate, elevation and steps of every activity in
 * one chart each, over distance or time.
 */
export function OverlayCharts({
  entries,
  stretched,
  onOpen,
}: {
  entries: CompareEntry[];
  stretched: boolean;
  onOpen: (id: number) => void;
}) {
  const [axis, setAxis] = useState<Axis>("distance");
  const series = useMemo(() => seriesOf(entries, stretched), [entries, stretched]);
  const mode = speedMode(entries.map((e) => e.view));
  const pace = (v: number) => `${formatPace(v)} /km`;
  const metrics: Metric[] = [
    mode === "pace"
      ? {
          title: t("Pace (/km)"),
          value: (p) => p.pace,
          format: pace,
          tick: formatPace,
          robust: true,
          reversed: true,
        }
      : {
          title: t("Speed (km/h)"),
          value: (p) => speedFromPace(p.pace),
          format: (v) => `${formatNumber(v, 1)} km/h`,
          tick: (v) => formatNumber(v),
          robust: true,
        },
    {
      title: t("GAP (/km)"),
      value: (p) => p.gap,
      format: pace,
      tick: formatPace,
      robust: true,
      reversed: true,
      onFoot: true,
    },
    {
      title: t("Heart rate (bpm)"),
      value: (p) => p.heartRate,
      format: (v) => `${formatNumber(v)} bpm`,
      tick: (v) => formatNumber(v),
    },
    {
      title: t("Elevation (m)"),
      value: (p) => p.elevation,
      format: (v) => `${formatNumber(v)} m`,
      tick: (v) => formatNumber(v),
    },
    {
      title: t("Steps per minute"),
      value: (p) => p.cadence,
      format: (v) => `${formatNumber(v)} spm`,
      tick: (v) => formatNumber(v),
      robust: true,
      onFoot: true,
    },
  ];
  const charts = metrics
    .map((metric) => {
      const used = metric.onFoot
        ? series.filter((s) => onFoot(entries.find((e) => e.key === s.key)?.view ?? "All"))
        : series;
      return { metric, rows: overlayRows(used, metric.value, axis) };
    })
    .filter(({ rows }) => rows.length > 0);
  if (!charts.length) return null;
  const toggle = (
    <div className="flex rounded-md border p-0.5">
      {AXES.map(({ axis: value, label }) => (
        <Button
          key={value}
          size="xs"
          variant={axis === value ? "secondary" : "ghost"}
          aria-pressed={axis === value}
          onClick={() => setAxis(value)}
        >
          {t(label)}
        </Button>
      ))}
    </div>
  );
  return (
    <ChartCard
      title={axis === "distance" ? t("Over distance") : t("Over time")}
      note={stretched ? t("On the segment, from its start.") : undefined}
      action={toggle}
    >
      <div className="grid gap-6 xl:grid-cols-2 *:min-w-0">
        {charts.map(({ metric, rows }) => (
          <div key={metric.title} className="grid gap-1">
            <div className="text-xs font-medium text-muted-foreground">{metric.title}</div>
            <LinesChart
              rows={rows}
              entries={entries}
              xKey="x"
              formatX={axis === "distance" ? (v) => formatKm(v, 1) : formatClock}
              formatY={metric.format}
              formatTick={metric.tick}
              robust={metric.robust}
              reversed={metric.reversed}
              onOpen={onOpen}
            />
          </div>
        ))}
      </div>
    </ChartCard>
  );
}

/**
 * The virtual race: how far each activity is behind (or ahead of) the fastest one at every point
 * of the distance they all cover.
 */
export function RaceChart({
  entries,
  stretched,
  onOpen,
}: {
  entries: CompareEntry[];
  stretched: boolean;
  onOpen: (id: number) => void;
}) {
  const race = useMemo(() => raceGaps(seriesOf(entries, stretched)), [entries, stretched]);
  if (!race) return null;
  const leader = entries.find((e) => e.key === race.leader);
  return (
    <ChartCard
      title={t("Virtual race")}
      note={t(
        "Time behind (+) or ahead (−) of the fastest, {{label}}, over the first {{distance}}, the distance all of them cover.",
        { label: leader?.label ?? "", distance: formatKm(race.distance) },
      )}
    >
      <LinesChart
        rows={race.rows}
        entries={entries}
        xKey="distance"
        formatX={(v) => formatKm(v)}
        formatY={formatGap}
        zeroLine
        onOpen={onOpen}
      />
    </ChartCard>
  );
}

/** Pace (or speed) of every split, the splits of all activities side by side. */
export function SplitsChart({
  entries,
  stretched,
  onOpen,
}: {
  entries: CompareEntry[];
  stretched: boolean;
  onOpen: (id: number) => void;
}) {
  const views = entries.map((e) => e.view);
  const mode = speedMode(views);
  const rows = splitRows(entries, mode);
  if (!rows.length) return null;
  if (!splitsComparable(views)) {
    return (
      <ChartCard title={t("Splits")}>
        <p className="text-sm text-muted-foreground">
          {t(
            "Rides are split every 5 km, runs and walks every kilometre, so they aren't compared.",
          )}
        </p>
      </ChartCard>
    );
  }
  const rides = views.every((view) => !onFoot(view));
  return (
    <ChartCard
      title={rides ? t("Splits per 5 km") : t("Splits per km")}
      note={stretched ? t("Splits are those of the whole activities.") : undefined}
    >
      <LinesChart
        rows={rows}
        entries={entries}
        xKey="split"
        category
        dots
        formatX={(v) => formatKm(rides ? v * 5 : v, 0)}
        formatY={(v) => (mode === "pace" ? `${formatPace(v)} /km` : `${formatNumber(v, 1)} km/h`)}
        formatTick={(v) => (mode === "pace" ? formatPace(v) : formatNumber(v))}
        reversed={mode === "pace"}
        onOpen={onOpen}
      />
    </ChartCard>
  );
}
