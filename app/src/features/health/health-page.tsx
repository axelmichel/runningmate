import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
  CartesianGrid,
  ComposedChart,
  Line,
  LineChart,
  ReferenceLine,
  Scatter,
  XAxis,
  YAxis,
} from "recharts";

import type { HealthHeartPoint } from "@/bindings/HealthHeartPoint";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { RangeInput } from "@/features/connectors/range";
import { ZoneBars } from "@/features/detail/zone-bars";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { DEFAULT_MONTHS, PRESET_MONTHS, lastMonths, periodBounds } from "./period";

const fitnessConfig = () =>
  ({
    fitness: { label: t("Fitness"), color: "var(--chart-1)" },
    fatigue: { label: t("Fatigue"), color: "var(--chart-3)" },
    form: { label: t("Form"), color: "var(--chart-5)" },
  }) satisfies ChartConfig;

const heartConfig = () =>
  ({
    Running: { label: t("Runs"), color: "var(--chart-1)" },
    Cycling: { label: t("Rides"), color: "var(--chart-3)" },
    Walking: { label: t("Walks"), color: "var(--chart-5)" },
    trend: { label: t("Trend"), color: "var(--chart-4)" },
  }) satisfies ChartConfig;

const TYPES = ["Running", "Cycling", "Walking"] as const;

/** Day string ("YYYY-MM-DD", UTC) as a Unix timestamp for the date formatter. */
const dayTimestamp = (day: string) => Date.parse(`${day}T12:00:00Z`) / 1000;

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <Card size="sm" className="gap-1">
      <CardContent>
        <div className="text-xs text-muted-foreground">{label}</div>
        <div className="text-xl font-semibold text-primary tabular-nums">{value}</div>
        {hint && <div className="text-xs text-muted-foreground">{hint}</div>}
      </CardContent>
    </Card>
  );
}

function PeriodPicker({
  value,
  onChange,
}: {
  value: RangeInput;
  onChange: (value: RangeInput) => void;
}) {
  const presets = PRESET_MONTHS.map((months) => ({ months, range: lastMonths(months) }));
  return (
    <div className="flex flex-wrap items-end gap-3">
      <div className="flex gap-1" role="group" aria-label={t("Period")}>
        {presets.map(({ months, range }) => {
          const active = range.from === value.from && range.to === value.to;
          return (
            <Button
              key={months}
              size="sm"
              variant={active ? "default" : "outline"}
              aria-pressed={active}
              onClick={() => onChange(range)}
            >
              {months === 12 ? t("1 year") : t("{{count}} month", { count: months })}
            </Button>
          );
        })}
      </div>
      {(["from", "to"] as const).map((key) => (
        <div key={key} className="grid gap-1.5">
          <Label htmlFor={`health-${key}`}>{key === "from" ? t("From") : t("To")}</Label>
          <Input
            id={`health-${key}`}
            type="date"
            className="h-8 w-40"
            value={value[key]}
            min={key === "to" ? value.from : undefined}
            max={key === "from" ? value.to : undefined}
            onChange={(e) => onChange({ ...value, [key]: e.target.value })}
          />
        </div>
      ))}
    </div>
  );
}

function FitnessCard({ fitness }: { fitness: { day: string }[] }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">{t("Fitness, fatigue and form")}</CardTitle>
      </CardHeader>
      <CardContent>
        {fitness.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("No training in this period.")}</p>
        ) : (
          <ChartContainer config={fitnessConfig()} className="aspect-auto h-64 w-full">
            <LineChart data={fitness}>
              <CartesianGrid vertical={false} />
              <XAxis
                dataKey="day"
                tickLine={false}
                axisLine={false}
                minTickGap={40}
                tickFormatter={(day: string) => formatDate(dayTimestamp(day))}
              />
              <YAxis
                width={40}
                tickLine={false}
                axisLine={false}
                tickFormatter={(v: number) => formatNumber(v)}
              />
              <ChartTooltip
                content={
                  <ChartTooltipContent
                    labelFormatter={(_, p) =>
                      formatDate(dayTimestamp(String(p?.[0]?.payload?.day)))
                    }
                  />
                }
              />
              <ChartLegend content={<ChartLegendContent />} />
              {(["fitness", "fatigue", "form"] as const).map((key) => (
                <Line
                  key={key}
                  dataKey={key}
                  type="monotone"
                  stroke={`var(--color-${key})`}
                  dot={false}
                  isAnimationActive={false}
                />
              ))}
            </LineChart>
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  );
}

function HeartRateCard({
  points,
  references,
}: {
  points: HealthHeartPoint[];
  references: [number, number, number] | null;
}) {
  // One scatter series per activity type, keyed by the type so each gets its colour and label.
  const series = TYPES.map((type) => ({
    type,
    data: points
      .filter((p) => p.viewMode === type)
      .map((p) => ({ date: p.date, [type]: p.avgHeartRate })),
  })).filter((s) => s.data.length > 0);
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">{t("Average heart rate per activity")}</CardTitle>
      </CardHeader>
      <CardContent>
        {points.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("No heart-rate data in this period.")}</p>
        ) : (
          <ChartContainer config={heartConfig()} className="aspect-auto h-64 w-full">
            <ComposedChart data={points}>
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
                tickFormatter={(v: number) => formatNumber(v)}
              />
              <ChartTooltip
                content={
                  <ChartTooltipContent
                    labelFormatter={(_, p) => formatDate(p?.[0]?.payload?.date)}
                  />
                }
              />
              <ChartLegend content={<ChartLegendContent />} />
              {references?.map((bpm, i) => (
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
              {series.map(({ type, data }) => (
                <Scatter
                  key={type}
                  name={type}
                  data={data}
                  dataKey={type}
                  fill={`var(--color-${type})`}
                  isAnimationActive={false}
                />
              ))}
              <Line
                dataKey="trend"
                type="monotone"
                stroke="var(--color-trend)"
                strokeWidth={2}
                dot={false}
                isAnimationActive={false}
              />
            </ComposedChart>
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  );
}

/** Fitness and heart rate over a chosen period (the last three months by default). */
export function HealthPage() {
  const [range, setRange] = useState<RangeInput>(() => lastMonths(DEFAULT_MONTHS));
  const bounds = periodBounds(range);
  const { data: report, error } = useQuery({
    queryKey: ["health", bounds?.from, bounds?.to],
    queryFn: () => api.healthReport(bounds!.from, bounds!.to),
    enabled: bounds !== null,
    placeholderData: (previous) => previous,
  });
  const latest = report?.fitness.at(-1);
  const heart = report?.heartRate ?? [];
  const averageHeartRate = heart.length
    ? heart.reduce((sum, p) => sum + p.avgHeartRate, 0) / heart.length
    : null;

  return (
    <div className="flex h-full flex-col overflow-y-auto p-4">
      <div className={cn("grid gap-4", !bounds && "opacity-60")}>
        <PeriodPicker value={range} onChange={setRange} />
        {error && <p className="text-sm break-words text-destructive">{String(error)}</p>}
        <div className="grid grid-cols-2 gap-3 xl:grid-cols-4">
          <Stat
            label={t("Activities")}
            value={formatNumber(report?.activities)}
            hint={t("{{count}} with heart rate", { count: heart.length })}
          />
          <Stat
            label={t("Average heart rate")}
            value={averageHeartRate === null ? "–" : `${formatNumber(averageHeartRate)} bpm`}
          />
          <Stat
            label={t("Fitness")}
            value={formatNumber(latest?.fitness, 1)}
            hint={latest && t("Fatigue {{value}}", { value: formatNumber(latest.fatigue, 1) })}
          />
          <Stat
            label={t("Form")}
            value={formatNumber(latest?.form, 1)}
            hint={latest && t("on {{date}}", { date: formatDate(dayTimestamp(latest.day)) })}
          />
        </div>
        <FitnessCard fitness={report?.fitness ?? []} />
        <HeartRateCard points={heart} references={report?.references ?? null} />
        <Card size="sm">
          <CardHeader>
            <CardTitle className="text-sm">{t("Time in heart-rate zones")}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-2 text-sm">
            {report && !report.zones && (
              <p className="text-muted-foreground">
                {t("Set your heart-rate zones in the user settings to see this.")}
              </p>
            )}
            {report?.zones && <ZoneBars zones={report.zones} />}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
