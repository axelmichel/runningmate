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
import { t } from "@/i18n";
import { formatDate } from "@/lib/format";

import { ZoneBars } from "./zone-bars";

const trendConfig = () =>
  ({
    avgHeartRate: { label: t("Activity"), color: "var(--chart-2)" },
    trend: { label: t("Trend"), color: "var(--chart-4)" },
  }) satisfies ChartConfig;

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
          <CardTitle className="text-sm">{t("Time in heart-rate zones")}</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-2 text-sm">
          {!isPending && !zones && (
            <p className="text-muted-foreground">
              {t("Set your heart-rate zones in the user settings to see this.")}
            </p>
          )}
          {zones && <ZoneBars zones={zones} />}
        </CardContent>
      </Card>
      <Card size="sm">
        <CardHeader>
          <CardTitle className="text-sm">
            {t("Heart-rate trend (90 days, same activity type)")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {!trend?.points.length ? (
            <p className="text-sm text-muted-foreground">No heart-rate data in this period.</p>
          ) : (
            <ChartContainer config={trendConfig()} className="aspect-auto h-56 w-full">
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
                      value: `${[t("Min"), t("Ideal"), t("Max")][i]} ${bpm}`,
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
