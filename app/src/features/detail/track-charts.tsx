import { memo, useMemo } from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  XAxis,
  YAxis,
} from "recharts";
import type { CategoricalChartFunc } from "recharts/types/chart/types";

import type { TrackPoint } from "@/bindings/TrackPoint";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { t } from "@/i18n";
import { formatNumber } from "@/lib/format";

import { speedFromPace } from "./track-position";

type ChartPoint = TrackPoint & { speed: number | null };

const chartConfig = () =>
  ({
    elevation: { label: t("Elevation (m)"), color: "var(--chart-2)" },
    heartRate: { label: t("Heart rate (bpm)"), color: "var(--chart-3)" },
    speed: { label: t("Speed (km/h)"), color: "var(--chart-4)" },
    cadence: { label: t("Steps per minute"), color: "var(--chart-5)" },
  }) satisfies ChartConfig;

/** Charts sharing this id show their tooltip at the same point. */
const SYNC_ID = "track";

/**
 * Elevation, heart rate, speed and (for runs) steps per minute over distance, below the map.
 * Hovering reports the point's index (`null` on leaving); `markKm` draws a line at a chosen
 * distance.
 */
export const TrackCharts = memo(function TrackCharts({
  points,
  showCadence,
  markKm,
  onHover,
}: {
  points: TrackPoint[];
  /** Shows the steps chart when the track has cadence. */
  showCadence: boolean;
  markKm: number | null;
  onHover: (index: number | null) => void;
}) {
  const config = chartConfig();
  const data = useMemo<ChartPoint[]>(
    () => points.map((p) => ({ ...p, speed: speedFromPace(p.pace) })),
    [points],
  );
  const hasHeartRate = points.some((p) => p.heartRate != null);
  const hasSpeed = data.some((p) => p.speed != null);
  const hasCadence = showCadence && points.some((p) => p.cadence != null);
  const onMouseMove: CategoricalChartFunc = (state) => {
    const index = Number(state.activeTooltipIndex);
    onHover(Number.isInteger(index) && index >= 0 && index < data.length ? index : null);
  };
  const common = {
    data,
    syncId: SYNC_ID,
    margin: { top: 4, right: 8, bottom: 0, left: 0 },
    onMouseMove,
    onMouseLeave: () => onHover(null),
  };
  const axis = (
    <XAxis
      dataKey="distance"
      type="number"
      domain={["dataMin", "dataMax"]}
      tickFormatter={(v: number) => `${formatNumber(v, 0)} km`}
      tickLine={false}
      axisLine={false}
    />
  );
  const tooltip = (
    <ChartTooltip
      content={
        <ChartTooltipContent
          labelFormatter={(_, payload) =>
            `${formatNumber((payload?.[0]?.payload as TrackPoint | undefined)?.distance, 2)} km`
          }
        />
      }
    />
  );
  const mark = markKm !== null && (
    <ReferenceLine x={markKm} stroke="var(--foreground)" strokeDasharray="3 3" />
  );
  const chart = "aspect-auto h-28 w-full";

  return (
    <div className="grid gap-1">
      <ChartContainer config={config} className={chart}>
        <AreaChart {...common}>
          <CartesianGrid vertical={false} />
          {axis}
          <YAxis
            width={44}
            tickLine={false}
            axisLine={false}
            domain={["dataMin - 10", "dataMax + 10"]}
            tickFormatter={(v: number) => formatNumber(v)}
          />
          {tooltip}
          {mark}
          <Area
            dataKey="elevation"
            type="monotone"
            stroke="var(--color-elevation)"
            fill="var(--color-elevation)"
            fillOpacity={0.25}
            isAnimationActive={false}
          />
        </AreaChart>
      </ChartContainer>
      {hasHeartRate && (
        <ChartContainer config={config} className={chart}>
          <LineChart {...common}>
            <CartesianGrid vertical={false} />
            {axis}
            <YAxis
              width={44}
              tickLine={false}
              axisLine={false}
              domain={["dataMin - 5", "dataMax + 5"]}
              tickFormatter={(v: number) => formatNumber(v)}
            />
            {tooltip}
            {mark}
            <Line
              dataKey="heartRate"
              type="monotone"
              stroke="var(--color-heartRate)"
              dot={false}
              strokeWidth={1.5}
              connectNulls
              isAnimationActive={false}
            />
          </LineChart>
        </ChartContainer>
      )}
      {hasSpeed && (
        <ChartContainer config={config} className={chart}>
          <LineChart {...common}>
            <CartesianGrid vertical={false} />
            {axis}
            <YAxis
              width={44}
              tickLine={false}
              axisLine={false}
              domain={[0, "dataMax"]}
              tickFormatter={(v: number) => formatNumber(v)}
            />
            {tooltip}
            {mark}
            <Line
              dataKey="speed"
              type="monotone"
              stroke="var(--color-speed)"
              dot={false}
              strokeWidth={1.5}
              connectNulls
              isAnimationActive={false}
            />
          </LineChart>
        </ChartContainer>
      )}
      {hasCadence && (
        <ChartContainer config={config} className={chart}>
          <LineChart {...common}>
            <CartesianGrid vertical={false} />
            {axis}
            <YAxis
              width={44}
              tickLine={false}
              axisLine={false}
              domain={["dataMin - 5", "dataMax + 5"]}
              tickFormatter={(v: number) => formatNumber(v)}
            />
            {tooltip}
            {mark}
            <Line
              dataKey="cadence"
              type="monotone"
              stroke="var(--color-cadence)"
              dot={false}
              strokeWidth={1.5}
              connectNulls
              isAnimationActive={false}
            />
          </LineChart>
        </ChartContainer>
      )}
    </div>
  );
});
