import { useMemo } from "react";
import { Area, CartesianGrid, ComposedChart, Line, XAxis, YAxis } from "recharts";

import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { t } from "@/i18n";
import { formatNumber } from "@/lib/format";

import { gradeSeries } from "./segment-math";

type ProfilePoint = { distance: number; elevation: number; grade: number };

const chartConfig = () =>
  ({
    elevation: { label: t("Elevation (m)"), color: "var(--chart-2)" },
    grade: { label: t("Grade (%)"), color: "var(--chart-4)" },
  }) satisfies ChartConfig;

/**
 * Elevation and grade over distance. `points` carry the distance in metres from the start;
 * `offsetKm` shifts the axis, e.g. to the position within an activity.
 */
export function SegmentProfile({
  points,
  offsetKm = 0,
  className = "h-48",
}: {
  points: { distance: number; elevation: number }[];
  offsetKm?: number;
  className?: string;
}) {
  const data = useMemo<ProfilePoint[]>(() => {
    const grades = gradeSeries(points);
    return points.map((p, i) => ({
      distance: offsetKm + p.distance / 1000,
      elevation: p.elevation,
      grade: grades[i],
    }));
  }, [points, offsetKm]);
  if (data.length < 2) return null;
  return (
    <ChartContainer config={chartConfig()} className={`aspect-auto w-full ${className}`}>
      <ComposedChart data={data}>
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey="distance"
          type="number"
          domain={["dataMin", "dataMax"]}
          tickFormatter={(v: number) => `${formatNumber(v, 1)} km`}
          tickLine={false}
          axisLine={false}
        />
        <YAxis
          yAxisId="elevation"
          width={40}
          tickLine={false}
          axisLine={false}
          domain={["dataMin - 5", "dataMax + 5"]}
          tickFormatter={(v: number) => formatNumber(v)}
        />
        <YAxis
          yAxisId="grade"
          orientation="right"
          width={40}
          tickLine={false}
          axisLine={false}
          tickFormatter={(v: number) => `${formatNumber(v)} %`}
        />
        <ChartTooltip
          content={
            <ChartTooltipContent
              labelFormatter={(_, payload) =>
                `${formatNumber((payload?.[0]?.payload as ProfilePoint | undefined)?.distance, 2)} km`
              }
              formatter={(value, name) => (
                <div className="flex w-full justify-between gap-4">
                  <span className="text-muted-foreground">
                    {chartConfig()[name as keyof ReturnType<typeof chartConfig>]?.label}
                  </span>
                  <span className="font-mono tabular-nums">
                    {formatNumber(value as number, name === "grade" ? 1 : 0)}
                  </span>
                </div>
              )}
            />
          }
        />
        <Area
          yAxisId="elevation"
          dataKey="elevation"
          type="monotone"
          stroke="var(--color-elevation)"
          fill="var(--color-elevation)"
          fillOpacity={0.25}
          isAnimationActive={false}
        />
        <Line
          yAxisId="grade"
          dataKey="grade"
          type="monotone"
          stroke="var(--color-grade)"
          strokeWidth={1.5}
          dot={false}
          isAnimationActive={false}
        />
      </ComposedChart>
    </ChartContainer>
  );
}
