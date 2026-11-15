import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { CartesianGrid, Line, LineChart, ReferenceArea, XAxis, YAxis } from "recharts";

import type { TrackPoint } from "@/bindings/TrackPoint";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDuration, formatNumber } from "@/lib/format";

import { useSeries } from "./use-series";
import { ZONE_COLORS, zoneGradient } from "./zone-gradient";

const chartConfig = () =>
  ({ heartRate: { label: t("Heart rate (bpm)"), color: "var(--chart-3)" } }) satisfies ChartConfig;

/** "1:05:00" → "1:05:00", "0:42:10" → "42:10". */
const clock = (seconds: number) => formatDuration(seconds).replace(/^0:/, "");

/**
 * Heart rate over time, the line coloured by zone over light zone bands. Uses the same track
 * series and zones as the map and the Zones tab, so they are only loaded once.
 */
export function ZoneChart({ activityId }: { activityId: number }) {
  const { data: points } = useSeries(activityId);
  const { data: zones, isPending } = useQuery({
    queryKey: ["zones", activityId],
    queryFn: () => api.activityHeartRateZones(activityId),
  });
  const line = useMemo(() => {
    const rates = (points ?? []).map((p) => p.heartRate).filter((hr): hr is number => hr != null);
    if (!rates.length) return null;
    const min = Math.min(...rates);
    const max = Math.max(...rates);
    const bounds = zones?.map((z) => z.maxBpm) ?? [];
    // The y range, and the zone bands cut to it (bands outside it are left out).
    const [low, high] = [min - 5, max + 5];
    const bands = (zones ?? [])
      .map((zone, i) => ({
        zone: zone.zone,
        y1: Math.max(low, i === 0 ? low : bounds[i - 1]),
        y2: Math.min(high, i === bounds.length - 1 ? high : zone.maxBpm),
      }))
      .filter((band) => band.y2 > band.y1);
    const gradient = zones ? zoneGradient(min, max, bounds) : [];
    return { min, max, low, high, bounds, bands, gradient };
  }, [points, zones]);
  if (!points || !line) return null;

  const id = `zone-line-${activityId}`;
  // A flat line can't take a gradient: one colour for its zone.
  const flatZone = line.bounds.findIndex((bound) => line.max <= bound);
  const stroke = !zones
    ? "var(--color-heartRate)"
    : line.gradient.length
      ? `url(#${id})`
      : ZONE_COLORS[flatZone === -1 ? 4 : flatZone];

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">{t("Heart rate by zone")}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-2">
        {!isPending && !zones && (
          <p className="text-sm text-muted-foreground">
            {t("Set your heart-rate zones in the user settings to see this.")}
          </p>
        )}
        <ChartContainer config={chartConfig()} className="aspect-auto h-56 w-full">
          <LineChart data={points} margin={{ top: 4, right: 32, bottom: 0, left: 0 }}>
            <defs>
              <linearGradient id={id} x1="0" y1="0" x2="0" y2="1">
                {line.gradient.flatMap(({ zone, from, to }) => [
                  <stop key={`${zone}-from`} offset={from} stopColor={ZONE_COLORS[zone - 1]} />,
                  <stop key={`${zone}-to`} offset={to} stopColor={ZONE_COLORS[zone - 1]} />,
                ])}
              </linearGradient>
            </defs>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="elapsed"
              type="number"
              domain={["dataMin", "dataMax"]}
              tickFormatter={clock}
              tickLine={false}
              axisLine={false}
              minTickGap={32}
            />
            <YAxis
              width={40}
              tickLine={false}
              axisLine={false}
              domain={[line.low, line.high]}
              allowDataOverflow
              tickFormatter={(v: number) => formatNumber(v)}
            />
            {line.bands.map(({ zone, y1, y2 }) => (
              <ReferenceArea
                key={zone}
                y1={y1}
                y2={y2}
                fill={ZONE_COLORS[zone - 1]}
                fillOpacity={0.12}
                strokeOpacity={0}
                label={{
                  value: `Z${zone}`,
                  position: "right",
                  fontSize: 10,
                  fill: "var(--muted-foreground)",
                }}
              />
            ))}
            <ChartTooltip
              content={
                <ChartTooltipContent
                  labelFormatter={(_, payload) => {
                    const point = payload?.[0]?.payload as TrackPoint | undefined;
                    return point ? clock(point.elapsed) : "";
                  }}
                />
              }
            />
            <Line
              dataKey="heartRate"
              type="monotone"
              stroke={stroke}
              strokeWidth={1.5}
              dot={false}
              connectNulls
              isAnimationActive={false}
            />
          </LineChart>
        </ChartContainer>
      </CardContent>
    </Card>
  );
}
