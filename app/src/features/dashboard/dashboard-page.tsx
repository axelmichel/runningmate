import { useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
import { GoalChips } from "@/features/goals/goal-chips";
import { FORM_SCALE, changeOf, formPercent, segmentOf, shareOf } from "@/features/health/scales";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatHours, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { ActivityCalendar } from "./activity-calendar";
import { ActivityFeed } from "./activity-feed";
import { calendarWeeks, dashboardMoment, periodStart } from "./dashboard";
import { PeriodNav } from "./period-nav";
import { SideStats } from "./side-stats";
import { VolumeChart } from "./volume-chart";

/** Photos in the top row. */
const PHOTOS = 24;

function RecentPhotos({ onOpen }: { onOpen: (id: number) => void }) {
  const { data: photos = [] } = useQuery({
    queryKey: ["recentPhotos", PHOTOS],
    queryFn: () => api.recentPhotos(PHOTOS),
  });
  if (photos.length === 0) return null;
  return (
    <div className="flex gap-2 overflow-x-auto pb-1">
      {photos.map(({ activityId, media }) => (
        <button
          key={media.id}
          type="button"
          title={t("Open details")}
          className="size-32 shrink-0 overflow-hidden rounded-lg bg-muted transition-opacity hover:opacity-85 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
          onClick={() => onOpen(activityId)}
        >
          <img
            src={convertFileSrc(media.filePath)}
            alt=""
            loading="lazy"
            className="size-full object-cover"
          />
        </button>
      ))}
    </div>
  );
}

function Box({
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
    <CardContent className="grid gap-1">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="text-2xl font-semibold text-primary tabular-nums">{value}</div>
      {hint && <div className="text-xs text-muted-foreground">{hint}</div>}
    </CardContent>
  );
  return (
    <Card size="sm" className={onClick ? "p-0" : undefined}>
      {onClick ? (
        <button
          type="button"
          className="h-full rounded-[inherit] py-3 text-left transition-colors hover:bg-muted/50 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
          onClick={onClick}
        >
          {content}
        </button>
      ) : (
        content
      )}
    </Card>
  );
}

/** Activities, fitness and form of the four weeks up to `moment`, and the training calendar. */
function Overview({
  moment,
  live,
  onOpen,
  onHealth,
}: {
  moment: Date;
  /** Whether `moment` is now, rather than a day picked further back. */
  live: boolean;
  onOpen: (id: number) => void;
  onHealth: () => void;
}) {
  const from = periodStart(moment);
  const to = Math.floor(moment.getTime() / 1000);
  const { data: summary } = useQuery({
    queryKey: ["activities", "summary", "All", "dashboard", from, to],
    queryFn: () => api.activitySummary("All", { minDate: from, maxDate: to }),
  });
  const { data: report } = useQuery({
    queryKey: ["health", from, to],
    queryFn: () => api.healthReport(from, to),
  });
  const calendarFrom = Math.floor(calendarWeeks(moment)[0][0].getTime() / 1000);
  const { data: calendar } = useQuery({
    queryKey: ["activities", "calendar", calendarFrom, to],
    queryFn: () =>
      api.listActivities({
        viewMode: "All",
        offset: 0,
        limit: 500,
        sortField: "date",
        sortDirection: "asc",
        filter: { minDate: calendarFrom, maxDate: to },
      }),
  });

  // As on the Health data page: fitness against the personal best, form against fitness.
  const latest = report?.fitness.at(-1);
  const first = report?.fitness.at(0);
  const peak = report?.fitnessPeak ?? null;
  const fitnessShare = latest && peak ? shareOf(latest.fitness, peak.fitness) : null;
  const change =
    latest && first && first !== latest ? changeOf(first.fitness, latest.fitness) : null;
  // Rounded first, so a small drop doesn't show as "-0 %".
  const fitnessChange = change === null ? null : Math.round(change) || 0;
  const formShare = latest ? formPercent(latest.form, latest.fitness) : null;

  return (
    <div className="grid grid-cols-3 gap-3">
      {/* The three numbers stacked beside the calendar. */}
      <div className="grid gap-3">
        <Box
          label={live ? t("Activities in the last 4 weeks") : t("Activities in these 4 weeks")}
          value={formatNumber(summary?.count)}
          hint={
            summary
              ? `${formatNumber(summary.distance, 1)} km · ${formatHours(summary.duration)}`
              : undefined
          }
        />
        <Box
          label={t("Fitness")}
          value={fitnessShare === null ? "–" : `${formatNumber(fitnessShare)} %`}
          hint={
            fitnessShare === null
              ? undefined
              : [
                  t("of your best"),
                  fitnessChange !== null &&
                    t("{{change}} % in 4 weeks", {
                      change: `${fitnessChange > 0 ? "+" : ""}${formatNumber(fitnessChange)}`,
                    }),
                ]
                  .filter(Boolean)
                  .join(" · ")
          }
          onClick={onHealth}
        />
        <Box
          label={t("Form")}
          value={formShare === null ? "–" : `${formatNumber(formShare)} %`}
          hint={formShare === null ? undefined : t(segmentOf(FORM_SCALE, formShare).label)}
          onClick={onHealth}
        />
      </div>
      <Card size="sm" className="col-span-2">
        <CardHeader>
          <CardTitle className="text-sm">{t("Training calendar")}</CardTitle>
        </CardHeader>
        <CardContent className="flex-1">
          <ActivityCalendar rows={calendar?.rows ?? []} onOpen={onOpen} today={moment} />
        </CardContent>
      </Card>
    </div>
  );
}

/**
 * The start page, after Strava's dashboard: latest photos, goals, the last four weeks at a glance,
 * the activity feed, and the profile statistics on the side.
 */
export function DashboardPage({
  dataDir,
  onHealth,
  onGoals,
}: {
  dataDir?: string;
  onHealth: () => void;
  onGoals: () => void;
}) {
  const [detailId, setDetailId] = useState<number | null>(null);
  const [today] = useState(() => new Date());
  // The last day shown, picked with the arrows or the date field; `null` follows today.
  const [day, setDay] = useState<Date | null>(null);
  const moment = dashboardMoment(day, today);
  const at = day ? Math.floor(moment.getTime() / 1000) : undefined;
  const [feedIds, setFeedIds] = useState<number[]>([]);
  const onLoaded = useCallback((ids: number[]) => setFeedIds(ids), []);
  // Previous/next in the detail window walk through the feed as far as it is loaded.
  const index = detailId === null ? -1 : feedIds.indexOf(detailId);
  const neighbour = (delta: -1 | 1) => {
    const id = index < 0 ? undefined : feedIds[index + delta];
    return id === undefined ? undefined : () => setDetailId(id);
  };

  return (
    <div className="h-full overflow-y-auto">
      <div
        className={cn(
          "flex gap-4 p-4",
          // WebKit paints Leaflet's GPU-composited tiles above the dialog's blurred backdrop
          // and even its content, whatever the z-index; the feed maps aren't needed behind it.
          detailId !== null && "[&_.leaflet-container]:invisible",
        )}
      >
        <div className="grid min-w-0 flex-1 content-start gap-4">
          <RecentPhotos onOpen={setDetailId} />
          <PeriodNav day={day} today={today} onChange={setDay} />
          <GoalChips onGoals={onGoals} at={at} />
          <Overview moment={moment} live={day === null} onOpen={setDetailId} onHealth={onHealth} />
          <VolumeChart today={moment} />
          <ActivityFeed onOpen={setDetailId} onLoaded={onLoaded} until={at} />
        </div>
        <aside className="w-80 shrink-0">
          {/* Stays in view while the feed scrolls, and scrolls itself when taller than the
              window. Padding with a matching negative margin keeps the cards' rings unclipped. */}
          <div className="sticky top-3 -m-1 max-h-[calc(100svh-var(--titlebar-height)-1.5rem)] overflow-y-auto p-1">
            <SideStats onOpen={setDetailId} />
          </div>
        </aside>
      </div>
      <ActivityDetailDialog
        activityId={detailId}
        onClose={() => setDetailId(null)}
        onPrevious={neighbour(-1)}
        onNext={neighbour(1)}
        dataDir={dataDir}
      />
    </div>
  );
}
