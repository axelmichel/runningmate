import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import type { ActivitySummary } from "@/bindings/ActivitySummary";
import type { BestPerformance } from "@/bindings/BestPerformance";
import type { ViewMode } from "@/bindings/ViewMode";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { VIEW_MODES } from "@/features/activities/view-mode";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatDuration, formatHours, formatNumber, formatPace } from "@/lib/format";

import { perWeek, periodStart, yearStart } from "./dashboard";

const SPORTS = VIEW_MODES.filter((v) => v.mode !== "All");

function Row({
  label,
  value,
  hint,
  onClick,
}: {
  label: string;
  value: string;
  hint?: string;
  onClick?: () => void;
}) {
  const content = (
    <>
      <span className="text-muted-foreground">
        {label}
        {hint && <span className="block text-xs">{hint}</span>}
      </span>
      <span className="font-semibold tabular-nums">{value}</span>
    </>
  );
  const className = "flex items-baseline justify-between gap-3 py-1.5 text-sm";
  return onClick ? (
    <button
      type="button"
      className={`${className} w-full rounded-sm text-left hover:bg-muted/50`}
      onClick={onClick}
    >
      {content}
    </button>
  ) : (
    <div className={className}>{content}</div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">{title}</CardTitle>
      </CardHeader>
      <CardContent className="grid divide-y">{children}</CardContent>
    </Card>
  );
}

/** Activities, distance, time and elevation; `weekly` labels them as per-week averages. */
function Totals({ summary, weekly }: { summary?: ActivitySummary; weekly?: boolean }) {
  const s = summary && (weekly ? perWeek(summary) : summary);
  const suffix = weekly ? ` / ${t("week")}` : "";
  return (
    <>
      <Row label={`${t("Activities")}${suffix}`} value={formatNumber(s?.count, weekly ? 1 : 0)} />
      <Row label={`${t("Distance")}${suffix}`} value={`${formatNumber(s?.distance, 1)} km`} />
      <Row
        label={`${t("Time")}${suffix}`}
        value={s ? (weekly ? formatDuration(s.duration) : formatHours(s.duration)) : "–"}
      />
      <Row label={`${t("Elevation")}${suffix}`} value={`${formatNumber(s?.elevationGain)} m`} />
    </>
  );
}

/** The fastest segment per distance; "YYYY-MM-DD HH:MM:SS+00:00" start times as stored. */
function records(performances: BestPerformance[]) {
  const best = new Map<number, BestPerformance>();
  for (const p of performances) {
    if (!best.has(p.distance) && p.pace > 0) best.set(p.distance, p);
  }
  return [...best.values()].sort((a, b) => a.distance - b.distance);
}

function recordDate(start: string | null): string {
  if (!start) return "";
  const time = Date.parse(start.replace(" ", "T"));
  return Number.isNaN(time) ? "" : formatDate(time / 1000);
}

/**
 * Strava's profile sidebar for one sport: the last four weeks per week, personal records, and
 * the totals of this year and of all time.
 */
export function SideStats({ onOpen }: { onOpen: (id: number) => void }) {
  const [sport, setSport] = useState<ViewMode>("Running");
  const [now] = useState(() => new Date());
  const summary = (key: string, minDate?: number) => ({
    queryKey: ["activities", "summary", sport, "dashboard", key],
    queryFn: () => api.activitySummary(sport, minDate === undefined ? {} : { minDate }),
  });
  const recent = useQuery(summary("weeks", periodStart(now)));
  const year = useQuery(summary("year", yearStart(now)));
  const allTime = useQuery(summary("all"));
  const { data: performances = [] } = useQuery({
    queryKey: ["bestPerformances", "top", sport],
    queryFn: () => api.topBestPerformances(sport),
  });
  const cycling = sport === "Cycling";

  return (
    <div className="grid gap-3">
      <Tabs value={sport} onValueChange={(value) => setSport(value as ViewMode)}>
        <TabsList className="w-full">
          {SPORTS.map(({ mode, label, icon: Icon }) => (
            <TabsTrigger key={mode} value={mode} aria-label={t(label)} title={t(label)}>
              <Icon />
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      <Section title={t("Last 4 weeks")}>
        <Totals summary={recent.data} weekly />
      </Section>
      <Section title={t("Personal records")}>
        {records(performances).map((p) => (
          <Row
            key={p.distance}
            label={`${formatNumber(p.distance)} km`}
            value={`${formatDuration(p.pace * p.distance * 60)} · ${
              cycling ? `${formatNumber(60 / p.pace, 1)} km/h` : `${formatPace(p.pace)} /km`
            }`}
            hint={recordDate(p.startTime)}
            onClick={() => onOpen(p.activityId)}
          />
        ))}
        {records(performances).length === 0 && (
          <p className="py-1.5 text-sm text-muted-foreground">{t("No records yet.")}</p>
        )}
        {performances.length > 0 && (
          <p className="pt-1.5 text-xs text-muted-foreground">
            {t("Fastest stretch of the distance within an activity.")}
          </p>
        )}
      </Section>
      <Section title={t("This year")}>
        <Totals summary={year.data} />
      </Section>
      <Section title={t("All time")}>
        <Totals summary={allTime.data} />
      </Section>
    </div>
  );
}
