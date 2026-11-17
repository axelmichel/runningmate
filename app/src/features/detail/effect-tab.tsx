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

import { basisNotes, effectDescription, effectLabel, formatRecovery } from "./training-effect";
import { ZoneChart } from "./zone-chart";

const fitnessConfig = () =>
  ({
    fitness: { label: t("Fitness"), color: "var(--chart-1)" },
    fatigue: { label: t("Fatigue"), color: "var(--chart-3)" },
    form: { label: t("Form"), color: "var(--chart-5)" },
  }) satisfies ChartConfig;

/** A training effect 0–5 as five steps filled up to the value, with its wording. */
function EffectScale({ title, value }: { title: string; value: number }) {
  return (
    <div className="min-w-56 flex-1">
      <div className="text-sm text-muted-foreground">{title}</div>
      <div className="flex items-baseline gap-2">
        <span className="text-3xl font-semibold text-primary tabular-nums">
          {formatNumber(value, 1)}
        </span>
        <span className="text-sm">{effectLabel(value)}</span>
      </div>
      <div
        className="mt-2 flex gap-1"
        role="meter"
        aria-valuemin={0}
        aria-valuemax={5}
        aria-valuenow={value}
        aria-label={title}
      >
        {Array.from({ length: 5 }, (_, i) => (
          <div key={i} className="h-2 flex-1 overflow-hidden rounded-sm bg-muted">
            <div
              className="h-full bg-primary"
              style={{
                width: `${Math.min(1, Math.max(0, value - i)) * 100}%`,
                opacity: 0.45 + (i / 4) * 0.55,
              }}
            />
          </div>
        ))}
      </div>
      <div className="mt-1 flex justify-between text-xs text-muted-foreground tabular-nums">
        {[0, 1, 2, 3, 4, 5].map((n) => (
          <span key={n}>{n}</span>
        ))}
      </div>
    </div>
  );
}

/** Twelve rising bars, filled up to the score (the Python `TrainingEffectChart`). */
function EffectMeter({ score }: { score: number }) {
  return (
    <div
      className="flex h-24 items-end gap-1.5"
      role="meter"
      aria-valuemin={1}
      aria-valuemax={12}
      aria-valuenow={score}
      aria-label={t("Effect score")}
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

/**
 * Training effect (EPOC) beside the 1–12 effect score, heart rate by zone (runs and rides) and
 * the fitness trend.
 */
export function EffectTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const { data: effect } = useQuery({
    queryKey: ["trainingEffect", activityId],
    queryFn: () => api.activityTrainingEffect(activityId),
  });
  const { data: score } = useQuery({
    queryKey: ["effectScore", activityId],
    queryFn: () => api.activityEffectScore(activityId),
  });
  const { data: fitness = [] } = useQuery({
    queryKey: ["fitness", activityId],
    queryFn: () => api.fitnessTrend(activityId),
  });

  return (
    <div className="grid gap-4">
      <div className="grid items-start gap-4 lg:grid-cols-2">
        <Card size="sm">
          <CardHeader>
            <CardTitle className="text-sm">{t("Training effect")}</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-4">
            {effect === null && (
              <p className="text-sm text-muted-foreground">
                {t("No heart rate recorded, so there is no training effect.")}
              </p>
            )}
            {effect && (
              <>
                <div className="flex flex-wrap gap-6">
                  <EffectScale title={t("Aerobic")} value={effect.aerobic} />
                  <EffectScale title={t("Anaerobic")} value={effect.anaerobic} />
                </div>
                <div className="flex flex-wrap gap-x-8 gap-y-1 text-sm">
                  <div>
                    <span className="text-muted-foreground">{t("Peak EPOC")}: </span>
                    <span className="tabular-nums">
                      {t("{{value}} ml/kg", { value: formatNumber(effect.peakEpoc) })}
                    </span>
                  </div>
                  <div>
                    <span className="text-muted-foreground">{t("Recovery time")}: </span>
                    <span className="tabular-nums">{formatRecovery(effect.recoveryHours)}</span>
                  </div>
                </div>
                <ul className="grid gap-0.5 text-xs text-muted-foreground">
                  {basisNotes(effect).map((note) => (
                    <li key={note}>{note}</li>
                  ))}
                </ul>
              </>
            )}
          </CardContent>
        </Card>
        <Card size="sm">
          <CardHeader>
            <CardTitle className="text-sm">{t("Effect score")}</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-wrap items-end gap-6">
            {score && <EffectMeter score={score.score} />}
            {score && (
              <div className="min-w-48 flex-1 text-sm">
                <div className="text-3xl font-semibold text-primary tabular-nums">
                  {score.score}
                  <span className="text-base font-normal text-muted-foreground"> / 12</span>
                </div>
                <p>{effectDescription(score.score)}</p>
                <p className="text-muted-foreground">
                  {t("To fully recover you should rest for {{time}}.", {
                    time: formatRecovery(score.recoveryHours),
                  })}
                </p>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
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
