import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Bar, BarChart, CartesianGrid, LabelList, XAxis, YAxis } from "recharts";

import { Card, CardAction, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ChartContainer, ChartTooltip, type ChartConfig } from "@/components/ui/chart";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import i18n, { t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/format";

import {
  CHART_SPORTS,
  chartBars,
  chartStarts,
  type ChartBar,
  type ChartGrouping,
  type ChartMetric,
} from "./dashboard";

const METRICS: { value: ChartMetric; label: string; unit: string; digits: number }[] = [
  { value: "distance", label: tk("Distance"), unit: "km", digits: 0 },
  { value: "duration", label: tk("Time"), unit: "h", digits: 1 },
  { value: "elevation", label: tk("Elevation"), unit: "m", digits: 0 },
];
const GROUPINGS: { value: ChartGrouping; label: string }[] = [
  { value: "week", label: tk("Weekly") },
  { value: "month", label: tk("Monthly") },
];
const STORAGE_KEY = "runningmate-dashboard-chart";

const config = () =>
  ({
    Running: { label: t("Runs"), color: "var(--sport-running)" },
    Cycling: { label: t("Rides"), color: "var(--sport-cycling)" },
    Walking: { label: t("Walks"), color: "var(--sport-walking)" },
  }) satisfies ChartConfig;

/** The highest sport with a value in a bar, where its total is written. */
function topSport(bar: ChartBar) {
  return [...CHART_SPORTS].reverse().find((sport) => bar[sport] > 0);
}

function stored(): { metric: ChartMetric; grouping: ChartGrouping } {
  try {
    const value = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    return {
      metric: METRICS.some((m) => m.value === value.metric) ? value.metric : "distance",
      grouping: value.grouping === "month" ? "month" : "week",
    };
  } catch {
    return { metric: "distance", grouping: "week" };
  }
}

function Toggle<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <Tabs value={value} onValueChange={(next) => onChange(next as T)}>
      <TabsList>
        {options.map((option) => (
          <TabsTrigger key={option.value} value={option.value}>
            {t(option.label)}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}

/**
 * Distance, time or elevation per week or month, stacked by sport, with the total on each bar.
 * The two toggles are remembered.
 */
export function VolumeChart({ today }: { today: Date }) {
  const [choice, setChoice] = useState(stored);
  const choose = (next: Partial<typeof choice>) =>
    setChoice((current) => {
      const value = { ...current, ...next };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
      return value;
    });
  // Twelve months reach further back than twelve weeks, so one query serves both.
  const from = Math.floor(chartStarts("month", today)[0].getTime() / 1000);
  const to = Math.floor(today.getTime() / 1000);
  const { data } = useQuery({
    queryKey: ["activities", "volume", from, to],
    queryFn: () =>
      api.listActivities({
        viewMode: "All",
        offset: 0,
        limit: 5000,
        sortField: "date",
        sortDirection: "asc",
        filter: { minDate: from, maxDate: to },
      }),
  });
  const metric = METRICS.find((m) => m.value === choice.metric)!;
  const bars = chartBars(data?.rows ?? [], choice.grouping, choice.metric, today);
  const label = new Intl.DateTimeFormat(
    i18n.language,
    choice.grouping === "week" ? { day: "2-digit", month: "2-digit" } : { month: "short" },
  );
  const value = (v: number) => `${formatNumber(v, metric.digits)} ${metric.unit}`;
  const names = config();

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">
          {t(metric.label)} · {choice.grouping === "week" ? t("per week") : t("per month")}
        </CardTitle>
        <CardAction className="flex flex-wrap justify-end gap-2">
          <Toggle
            value={choice.metric}
            options={METRICS}
            onChange={(next) => choose({ metric: next })}
          />
          <Toggle
            value={choice.grouping}
            options={GROUPINGS}
            onChange={(next) => choose({ grouping: next })}
          />
        </CardAction>
      </CardHeader>
      <CardContent>
        <ChartContainer config={names} className="aspect-auto h-64 w-full">
          <BarChart data={bars} margin={{ top: 18 }}>
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey={(bar: ChartBar) => label.format(bar.start)}
              tickLine={false}
              axisLine={false}
              interval={0}
              fontSize={11}
            />
            <YAxis
              width={44}
              tickLine={false}
              axisLine={false}
              tickFormatter={(v: number) => formatNumber(v)}
            />
            <ChartTooltip
              cursor={{ fill: "var(--muted)", opacity: 0.5 }}
              content={({ active, payload }) => {
                const bar = payload?.[0]?.payload as ChartBar | undefined;
                if (!active || !bar) return null;
                return (
                  <div className="grid min-w-36 gap-1 rounded-lg border bg-background px-2.5 py-1.5 text-xs shadow-xl">
                    <div className="font-medium">
                      {choice.grouping === "week"
                        ? t("Week of {{date}}", { date: label.format(bar.start) })
                        : new Intl.DateTimeFormat(i18n.language, {
                            month: "long",
                            year: "numeric",
                          }).format(bar.start)}
                    </div>
                    {[...CHART_SPORTS].reverse().map((sport) => (
                      <div key={sport} className="flex items-center justify-between gap-3">
                        <span className="flex items-center gap-1.5 text-muted-foreground">
                          <span
                            className="size-2.5 rounded-[2px]"
                            style={{ background: `var(--color-${sport})` }}
                          />
                          {names[sport].label}
                        </span>
                        <span className="font-mono tabular-nums">{value(bar[sport])}</span>
                      </div>
                    ))}
                    <div className="flex justify-between gap-3 border-t pt-1 font-medium">
                      <span>{t("Total")}</span>
                      <span className="font-mono tabular-nums">{value(bar.total)}</span>
                    </div>
                  </div>
                );
              }}
            />
            {CHART_SPORTS.map((sport) => (
              <Bar
                key={sport}
                dataKey={sport}
                stackId="sports"
                fill={`var(--color-${sport})`}
                // A thin line in the card colour separates the stacked sports.
                stroke="var(--card)"
                strokeWidth={1}
                isAnimationActive={false}
              >
                {/* The total sits on the topmost sport of each bar. */}
                <LabelList
                  dataKey={(bar: ChartBar) => (topSport(bar) === sport ? bar.total : null)}
                  position="top"
                  fontSize={10}
                  className="fill-muted-foreground"
                  formatter={(v: unknown) =>
                    typeof v === "number" ? formatNumber(v, metric.digits) : ""
                  }
                />
              </Bar>
            ))}
          </BarChart>
        </ChartContainer>
        {/* In stacking order, bottom first. */}
        <div className="flex justify-center gap-4 pt-2 text-xs text-muted-foreground">
          {CHART_SPORTS.map((sport) => (
            <span key={sport} className="flex items-center gap-1.5">
              <span
                className="size-2.5 rounded-[2px]"
                style={{ background: `var(--sport-${sport.toLowerCase()})` }}
              />
              {names[sport].label}
            </span>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
