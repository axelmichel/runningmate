import { useQuery } from "@tanstack/react-query";
import { CartesianGrid, Line, LineChart, XAxis, YAxis } from "recharts";

import type { ViewMode } from "@/bindings/ViewMode";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { api } from "@/lib/api";
import { t } from "@/i18n";
import { formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { effectDescription, formatRecovery } from "./training-effect";
import { ZoneChart } from "./zone-chart";

const fitnessConfig = () =>
  ({
    fitness: { label: t("Fitness"), color: "var(--chart-1)" },
    fatigue: { label: t("Fatigue"), color: "var(--chart-3)" },
    form: { label: t("Form"), color: "var(--chart-5)" },
  }) satisfies ChartConfig;

/** Twelve rising bars, filled up to the score (the Python `TrainingEffectChart`). */
function EffectMeter({ score }: { score: number }) {
  return (
    <div
      className="flex h-24 items-end gap-1.5"
      role="meter"
      aria-valuemin={1}
      aria-valuemax={12}
      aria-valuenow={score}
      aria-label={t("Training effect")}
    >
      {Array.from({ length: 12 }, (_, i) => (
        <div
          key={i}
          className={cn("w-4 rounded-sm", i < score ? "bg-primary" : "bg-muted")}
          style={{
            height: `${25 + (i / 11) * 75}%`,
            opacity: i < score ? 0.45 + (i / 11) * 0.55 : 1,
          }}
        />
      ))}
    </div>
  );
}

/** Training effect, heart rate by zone (runs and rides) and the fitness trend. */
export function EffectTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const { data: effect } = useQuery({
    queryKey: ["trainingEffect", activityId],
    queryFn: () => api.activityTrainingEffect(activityId),
  });
  const { data: fitness = [] } = useQuery({
    queryKey: ["fitness", activityId],
    queryFn: () => api.fitnessTrend(activityId),
  });

  return (
    <div className="grid gap-4">
      <Card size="sm">
        <CardHeader>
          <CardTitle className="text-sm">{t("Training effect")}</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap items-end gap-6">
          {effect && <EffectMeter score={effect.score} />}
          {effect && (
            <div className="min-w-60 flex-1 text-sm">
              <div className="text-3xl font-semibold text-primary tabular-nums">{effect.score}</div>
              <p>{effectDescription(effect.score)}</p>
              <p className="text-muted-foreground">
                {t("To fully recover you should rest for {{time}}.", {
                  time: formatRecovery(effect.recoveryHours),
                })}
              </p>
            </div>
          )}
        </CardContent>
      </Card>
      {(view === "Running" || view === "Cycling") && <ZoneChart activityId={activityId} />}
      <Card size="sm">
        <CardHeader>
          <CardTitle className="text-sm">{t("Fitness trend (90 days)")}</CardTitle>
        </CardHeader>
        <CardContent>
          {fitness.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              {t("No training in the 90 days before this activity.")}
            </p>
          ) : (
            <ChartContainer config={fitnessConfig()} className="aspect-auto h-56 w-full">
              <LineChart data={fitness}>
                <CartesianGrid vertical={false} />
                <XAxis dataKey="day" tickLine={false} axisLine={false} minTickGap={32} />
                <YAxis
                  width={40}
                  tickLine={false}
                  axisLine={false}
                  tickFormatter={(v: number) => formatNumber(v)}
                />
                <ChartTooltip content={<ChartTooltipContent />} />
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
    </div>
  );
}
