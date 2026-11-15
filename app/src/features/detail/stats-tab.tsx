import { Area, AreaChart, CartesianGrid, Line, LineChart, XAxis, YAxis } from "recharts";

import type { TrackPoint } from "@/bindings/TrackPoint";
import type { ViewMode } from "@/bindings/ViewMode";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import { formatNumber, formatPace } from "@/lib/format";
import { t } from "@/i18n";

import { useSeries } from "./use-series";

const chartConfig = () =>
  ({
    elevation: { label: t("Elevation (m)"), color: "var(--chart-2)" },
    heartRate: { label: t("Heart rate (bpm)"), color: "var(--chart-3)" },
    pace: { label: t("Pace (/km)"), color: "var(--chart-4)" },
    cadence: { label: t("Steps per minute"), color: "var(--chart-5)" },
  }) satisfies ChartConfig;

const distanceAxis = (
  <XAxis
    dataKey="distance"
    type="number"
    domain={["dataMin", "dataMax"]}
    tickFormatter={(v: number) => `${formatNumber(v, 0)} km`}
    tickLine={false}
    axisLine={false}
  />
);

function ChartCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">{title}</CardTitle>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}

/**
 * Elevation, heart rate, pace and (for runs) steps per minute over distance (the Python
 * statistics page).
 */
export function StatsTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const { data: points, isPending } = useSeries(activityId);
  const config = chartConfig();
  if (isPending) return <Skeleton className="h-96 w-full" />;
  if (!points) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("No archived TCX file for this activity.")}
      </p>
    );
  }
  const hasHeartRate = points.some((p) => p.heartRate != null);
  const hasPace = points.some((p) => p.pace != null);
  const hasCadence = view === "Running" && points.some((p) => p.cadence != null);
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
  return (
    <div className="grid gap-4">
      <ChartCard title={t("Elevation")}>
        <ChartContainer config={config} className="aspect-auto h-48 w-full">
          <AreaChart data={points}>
            <CartesianGrid vertical={false} />
            {distanceAxis}
            <YAxis
              width={40}
              tickLine={false}
              axisLine={false}
              domain={["dataMin - 10", "dataMax + 10"]}
              tickFormatter={(v: number) => formatNumber(v)}
            />
            {tooltip}
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
      </ChartCard>
      {hasHeartRate && (
        <ChartCard title={t("Heart rate")}>
          <ChartContainer config={config} className="aspect-auto h-48 w-full">
            <LineChart data={points}>
              <CartesianGrid vertical={false} />
              {distanceAxis}
              <YAxis
                width={40}
                tickLine={false}
                axisLine={false}
                domain={["dataMin - 5", "dataMax + 5"]}
              />
              {tooltip}
              <Line
                dataKey="heartRate"
                type="monotone"
                stroke="var(--color-heartRate)"
                dot={false}
                connectNulls
                isAnimationActive={false}
              />
            </LineChart>
          </ChartContainer>
        </ChartCard>
      )}
      {hasPace && (
        <ChartCard title={t("Pace")}>
          <ChartContainer config={config} className="aspect-auto h-48 w-full">
            <LineChart data={points}>
              <CartesianGrid vertical={false} />
              {distanceAxis}
              {/* Faster (lower) paces at the top. */}
              <YAxis
                width={40}
                reversed
                tickLine={false}
                axisLine={false}
                domain={["dataMin", "dataMax"]}
                tickFormatter={(v: number) => formatPace(v)}
              />
              <ChartTooltip
                content={
                  <ChartTooltipContent
                    formatter={(value) => `${formatPace(Number(value))} /km`}
                    labelFormatter={(_, payload) =>
                      `${formatNumber((payload?.[0]?.payload as TrackPoint | undefined)?.distance, 2)} km`
                    }
                  />
                }
              />
              <Line
                dataKey="pace"
                type="monotone"
                stroke="var(--color-pace)"
                dot={false}
                connectNulls
                isAnimationActive={false}
              />
            </LineChart>
          </ChartContainer>
        </ChartCard>
      )}
      {hasCadence && (
        <ChartCard title={t("Steps per minute")}>
          <ChartContainer config={config} className="aspect-auto h-48 w-full">
            <LineChart data={points}>
              <CartesianGrid vertical={false} />
              {distanceAxis}
              <YAxis
                width={40}
                tickLine={false}
                axisLine={false}
                domain={["dataMin - 5", "dataMax + 5"]}
                tickFormatter={(v: number) => formatNumber(v)}
              />
              {tooltip}
              <Line
                dataKey="cadence"
                type="monotone"
                stroke="var(--color-cadence)"
                dot={false}
                connectNulls
                isAnimationActive={false}
              />
            </LineChart>
          </ChartContainer>
        </ChartCard>
      )}
    </div>
  );
}
