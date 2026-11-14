import { useQuery } from "@tanstack/react-query";
import { CartesianGrid, ComposedChart, Line, ReferenceLine, Scatter, XAxis, YAxis } from "recharts";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { api } from "@/lib/api";
import { formatDate, formatDuration, formatNumber } from "@/lib/format";

/** Zone colours from dark to light, after `ui/widget_heart_rate_zones.py`. */
const ZONE_COLORS = ["#35193e", "#701f57", "#ad1759", "#f37651", "#f6b48f"];

const trendConfig = {
  avgHeartRate: { label: "Activity", color: "var(--chart-2)" },
  trend: { label: "Trend", color: "var(--chart-4)" },
} satisfies ChartConfig;

export function ZonesTab({ activityId }: { activityId: number }) {
  const { data: zones, isPending } = useQuery({
    queryKey: ["zones", activityId],
    queryFn: () => api.activityHeartRateZones(activityId),
  });
  const { data: trend } = useQuery({
    queryKey: ["heartRateTrend", activityId],
    queryFn: () => api.heartRateTrend(activityId),
  });

  return (
    <div className="grid gap-4">
      <Card size="sm">
        <CardHeader>
          <CardTitle className="text-sm">Time in heart-rate zones</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-2 text-sm">
          {!isPending && !zones && (
            <p className="text-muted-foreground">
              Set your heart-rate zones in the user settings to see this.
            </p>
          )}
          {zones?.map((zone) => (
            <div key={zone.zone} className="grid grid-cols-[9rem_1fr_6rem] items-center gap-3">
              <span>
                Zone {zone.zone}{" "}
                <span className="text-muted-foreground">
                  ({zone.minBpm}–{zone.maxBpm})
                </span>
              </span>
              <div className="h-2.5 overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full rounded-full"
                  style={{ width: `${zone.percent}%`, backgroundColor: ZONE_COLORS[zone.zone - 1] }}
                />
              </div>
              <span className="text-right tabular-nums">
                {formatNumber(zone.percent)} % · {formatDuration(zone.seconds).replace(/^0:/, "")}
              </span>
            </div>
          ))}
        </CardContent>
      </Card>
      <Card size="sm">
        <CardHeader>
          <CardTitle className="text-sm">Heart-rate trend (90 days, same activity type)</CardTitle>
        </CardHeader>
        <CardContent>
          {!trend?.points.length ? (
            <p className="text-sm text-muted-foreground">No heart-rate data in this period.</p>
          ) : (
            <ChartContainer config={trendConfig} className="aspect-auto h-56 w-full">
              <ComposedChart data={trend.points}>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="date"
                  type="number"
                  domain={["dataMin", "dataMax"]}
                  tickFormatter={(v: number) => formatDate(v)}
                  tickLine={false}
                  axisLine={false}
                  minTickGap={40}
                />
                <YAxis
                  width={40}
                  tickLine={false}
                  axisLine={false}
                  domain={["dataMin - 10", "dataMax + 10"]}
                />
                <ChartTooltip
                  content={
                    <ChartTooltipContent
                      labelFormatter={(_, p) => formatDate(p?.[0]?.payload?.date)}
                    />
                  }
                />
                {trend.references?.map((bpm, i) => (
                  <ReferenceLine
                    key={i}
                    y={bpm}
                    stroke="var(--muted-foreground)"
                    strokeDasharray="4 4"
                    label={{
                      value: `${["Min", "Ideal", "Max"][i]} ${bpm}`,
                      position: "insideTopLeft",
                      fontSize: 10,
                    }}
                  />
                ))}
                <Scatter
                  dataKey="avgHeartRate"
                  fill="var(--color-avgHeartRate)"
                  isAnimationActive={false}
                />
                <Line
                  dataKey="trend"
                  type="monotone"
                  stroke="var(--color-trend)"
                  dot={false}
                  isAnimationActive={false}
                />
              </ComposedChart>
            </ChartContainer>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
