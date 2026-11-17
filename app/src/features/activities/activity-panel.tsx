import { useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Download,
  Droplets,
  ExternalLink,
  Images,
  MapPin,
  Maximize2,
  MessageSquareText,
  Sparkles,
  Wind,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { GoalStar } from "@/features/goals/goal-star";
import { useGoalHits } from "@/features/goals/use-goals";
import { trophyCount } from "@/features/segments/segment-math";
import { TrophyBadge } from "@/features/segments/trophy";
import { useSegmentEfforts } from "@/features/segments/use-segment-efforts";
import { SessionBadge } from "@/features/training/session-badge";
import { useActivitySession } from "@/features/training/use-training";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import {
  formatDateTime,
  formatDuration,
  formatMinSec,
  formatNumber,
  formatPace,
} from "@/lib/format";

import { hiddenBoxes, type PanelBox } from "./panel-boxes";
import { weatherLabel } from "./weather";
import { WeatherIcon } from "./weather-icon";

function Metric({
  label,
  value,
  align,
  title,
}: {
  label: string;
  value: string;
  align?: "center" | "right";
  title?: string;
}) {
  return (
    <div
      className={align === "right" ? "text-right" : align === "center" ? "text-center" : undefined}
      title={title}
    >
      <div className="text-lg font-semibold text-primary tabular-nums">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

/** White-on-transparent route image written by the Python importer. */
function TrackImage({ dataDir, fileId }: { dataDir: string; fileId: string }) {
  const [failed, setFailed] = useState(false);
  if (failed) return null;
  return (
    <div className="rounded-md bg-[#35193e] p-3">
      <img
        src={convertFileSrc(`${dataDir}/images/${fileId}_track.png`)}
        alt={t("Route")}
        className="mx-auto h-40 object-contain"
        onError={() => setFailed(true)}
      />
    </div>
  );
}

/** Route shape drawn from the archived TCX; falls back to the Python image without one. */
function Route({ activity, dataDir }: { activity: ActivityDetail; dataDir?: string }) {
  const { data: route, isPending } = useQuery({
    queryKey: ["route", activity.id],
    queryFn: () => api.activityRoute(activity.id),
    staleTime: Infinity,
  });
  if (isPending) return <Skeleton className="h-40 w-full" />;
  if (!route) {
    return dataDir && activity.fileId ? (
      <TrackImage dataDir={dataDir} fileId={activity.fileId} />
    ) : null;
  }
  const exportImage = async () => {
    const path = await save({
      defaultPath: `${activity.fileId ?? `activity-${activity.id}`}_track.png`,
      filters: [
        { name: "PNG", extensions: ["png"] },
        { name: "SVG", extensions: ["svg"] },
      ],
    });
    if (!path) return;
    try {
      await api.exportRouteImage(activity.id, path);
      toast.success(t("Route image saved"));
    } catch (error) {
      toast.error(t("Could not save the route image: {{error}}", { error: String(error) }));
    }
  };
  const pad = 0.04;
  const points = route.points.map(([x, y]) => `${x.toFixed(4)},${y.toFixed(4)}`).join(" ");
  return (
    <div className="group relative">
      <svg
        role="img"
        aria-label={t("Route")}
        viewBox={`${-pad} ${-pad} ${route.width + 2 * pad} ${route.height + 2 * pad}`}
        className="mx-auto h-40 w-full text-primary"
        preserveAspectRatio="xMidYMid meet"
      >
        <polyline
          points={points}
          fill="none"
          stroke="currentColor"
          strokeWidth={0.012}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      {/* White on transparent, for sharing or laying over photos (Python's _track.png). */}
      <Button
        variant="secondary"
        size="icon-sm"
        aria-label={t("Save route image…")}
        title={t("Save route image…")}
        className="absolute top-0 right-0 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        onClick={() => void exportImage()}
      >
        <Download />
      </Button>
    </div>
  );
}

function Overview({
  activity,
  dataDir,
  onOpenDetails,
  shown,
}: {
  activity: ActivityDetail;
  dataDir?: string;
  onOpenDetails?: (id: number) => void;
  shown: (box: PanelBox) => boolean;
}) {
  const gap = activity.viewMode !== "Cycling" ? activity.avgGap : null;
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle data-private className="leading-snug">
          {activity.title}
        </CardTitle>
        {onOpenDetails && (
          <CardAction>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={t("Open details")}
              title={t("Open details")}
              onClick={() => onOpenDetails(activity.id)}
            >
              <Maximize2 />
            </Button>
          </CardAction>
        )}
        <div className="text-xs text-muted-foreground">
          {formatDateTime(activity.date)} · {t(activity.typeDetail || activity.activityType || "")}
          {activity.location && (
            <span className="flex items-center gap-1" title={activity.location}>
              <MapPin className="size-3 shrink-0" />
              <span data-private className="truncate">
                {activity.location}
              </span>
            </span>
          )}
        </div>
      </CardHeader>
      <CardContent className="grid gap-4">
        {/* Grade-adjusted pace beside the pace, for runs and walks. */}
        <div className={`grid gap-2 ${gap ? "grid-cols-3" : "grid-cols-2"}`}>
          <Metric label={t("Duration")} value={formatDuration(activity.duration)} />
          <Metric
            label={t("Pace (/km)")}
            value={formatMinSec(activity.avgPace)}
            align={gap ? "center" : "right"}
          />
          {gap && (
            <Metric
              label={t("GAP (/km)")}
              value={formatMinSec(gap)}
              align="right"
              title={t("Grade-adjusted pace: the pace on the flat for the same effort")}
            />
          )}
        </div>
        {shown("map") && <Route activity={activity} dataDir={dataDir} />}
        <div className="grid grid-cols-2 gap-2">
          <Metric label={t("Distance")} value={`${formatNumber(activity.distance, 2)} km`} />
          <Metric
            label={t("Elevation")}
            value={`${formatNumber(activity.elevationGain)} m`}
            align="right"
          />
        </div>
        {shown("links") && <Links activityId={activity.id} />}
      </CardContent>
    </Card>
  );
}

/**
 * Goal star, training-session marker and segment trophy in one row of their own, labelled with
 * the number of goals, the kind of session and the number of trophies, details on hover; nothing
 * when the activity has none of them.
 */
function AchievementsCard({ activityId }: { activityId: number }) {
  const { data: hits = [] } = useGoalHits(activityId);
  const { data: session } = useActivitySession(activityId);
  const { data: efforts = [] } = useSegmentEfforts(activityId);
  if (hits.length === 0 && !session && trophyCount(efforts) === 0) return null;
  return (
    <Card size="sm">
      <CardContent className="flex flex-wrap items-center gap-2">
        <GoalStar activityId={activityId} labelled />
        <SessionBadge activityId={activityId} labelled />
        <TrophyBadge activityId={activityId} labelled />
      </CardContent>
    </Card>
  );
}

/** Services this activity is linked to, e.g. "Garmin Connect · uploaded" with a link. */
function Links({ activityId }: { activityId: number }) {
  const { data: links = [] } = useQuery({
    queryKey: ["links", activityId],
    queryFn: () => api.activityLinks(activityId),
  });
  const garmin = links.find((link) => link.connector === "garmin");
  if (!garmin) return null;
  return (
    <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
      <span>Garmin Connect · {garmin.direction === "import" ? t("imported") : t("uploaded")}</span>
      {garmin.remoteId && (
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("Open in Garmin Connect")}
          title={t("Open in Garmin Connect")}
          onClick={() =>
            void openUrl(`https://connect.garmin.com/modern/activity/${garmin.remoteId}`)
          }
        >
          <ExternalLink />
        </Button>
      )}
    </div>
  );
}

function Excerpt({ icon: Icon, label, text }: { icon: LucideIcon; label: string; text: string }) {
  return (
    <div className="flex gap-2">
      <Icon className="mt-0.5 size-4 shrink-0 text-primary" aria-label={label} />
      <p data-private className="line-clamp-3 text-sm whitespace-pre-line">
        {text}
      </p>
    </div>
  );
}

/**
 * Photos, comment and AI summary at a glance, for the sidebar; opens the detail window, where
 * they are in full (so the detail window's own panel leaves this out).
 */
function NotesCard({
  activity,
  onOpenDetails,
}: {
  activity: ActivityDetail;
  onOpenDetails: (id: number) => void;
}) {
  const { data: media = [] } = useQuery({
    queryKey: ["media", activity.id],
    queryFn: () => api.listMedia(activity.id),
  });
  const { data: summary } = useQuery({
    queryKey: ["aiSummary", activity.id],
    queryFn: () => api.getAiSummary(activity.id),
  });
  const photos = media.filter((m) => m.mediaType === "image" && !m.missing).length;
  const comment = activity.comment?.trim();
  if (!photos && !comment && !summary) return null;

  return (
    <Card size="sm" className="p-0">
      <button
        type="button"
        title={t("Open details")}
        className="rounded-[inherit] py-3 text-left transition-colors hover:bg-muted/50 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={() => onOpenDetails(activity.id)}
      >
        <CardContent className="grid gap-2.5">
          {photos > 0 && (
            <div className="flex items-center gap-2 text-sm">
              <Images className="size-4 shrink-0 text-primary" />
              {t("{{count}} photo", { count: photos })}
            </div>
          )}
          {comment && <Excerpt icon={MessageSquareText} label={t("Comment")} text={comment} />}
          {summary && <Excerpt icon={Sparkles} label={t("Summary")} text={summary.text} />}
        </CardContent>
      </button>
    </Card>
  );
}

function WeatherCard({ activityId }: { activityId: number }) {
  const { data: weather } = useQuery({
    queryKey: ["weather", activityId],
    queryFn: () => api.getWeather(activityId),
  });
  if (!weather) return null;
  const avg =
    weather.maxTemp != null && weather.minTemp != null
      ? (weather.maxTemp + weather.minTemp) / 2
      : (weather.maxTemp ?? weather.minTemp);
  return (
    <Card size="sm">
      <CardContent className="flex items-center gap-3">
        <WeatherIcon code={weather.weatherCode} className="size-9 shrink-0 text-primary" />
        <div className="min-w-0 flex-1">
          <div className="font-semibold tabular-nums">{formatNumber(avg, 1)} °C</div>
          <div className="truncate text-xs text-muted-foreground">
            {weatherLabel(weather.weatherCode)}
          </div>
        </div>
        <div className="grid gap-0.5 text-xs text-muted-foreground tabular-nums">
          <span className="flex items-center gap-1">
            <Wind className="size-3" />
            {formatNumber(weather.maxWindSpeed)} km/h
          </span>
          <span className="flex items-center gap-1">
            <Droplets className="size-3" />
            {formatNumber(weather.precipitation, 1)} mm
          </span>
        </div>
      </CardContent>
    </Card>
  );
}

function BestPerformances({ activityId }: { activityId: number }) {
  const { data: performances } = useQuery({
    queryKey: ["bestPerformances", activityId],
    queryFn: () => api.activityBestPerformances(activityId),
  });
  if (!performances?.length) return null;
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">{t("Best segments")}</CardTitle>
      </CardHeader>
      <CardContent className="grid divide-y text-sm">
        {performances.map((p) => (
          <div key={p.distance} className="flex justify-between py-1.5">
            <span>{formatNumber(p.distance)} km</span>
            <span className="font-semibold text-primary tabular-nums">
              {formatPace(p.pace)} /km
            </span>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

/** Right-hand panel for the active (or latest) activity, like the Python main window. */
export function ActivityPanel({
  activityId,
  dataDir,
  onOpenDetails,
}: {
  activityId?: number;
  dataDir?: string;
  /** Shows a button that opens the detail window. */
  onOpenDetails?: (id: number) => void;
}) {
  const { data: activity, isPending } = useQuery({
    queryKey: ["activity", activityId],
    queryFn: () => api.getActivity(activityId!),
    enabled: activityId != null,
  });
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });

  if (activityId == null) {
    return <p className="p-2 text-sm text-muted-foreground">{t("No activity yet.")}</p>;
  }
  if (isPending || !activity) {
    return <Skeleton className="h-80 w-full" />;
  }
  const hidden = hiddenBoxes(settings, activity.viewMode);
  const shown = (box: PanelBox) => !hidden.includes(box);
  return (
    <div className="grid content-start gap-3">
      <Overview activity={activity} dataDir={dataDir} onOpenDetails={onOpenDetails} shown={shown} />
      {shown("achievements") && <AchievementsCard activityId={activity.id} />}
      {onOpenDetails && shown("notes") && (
        <NotesCard activity={activity} onOpenDetails={onOpenDetails} />
      )}
      {shown("weather") && <WeatherCard activityId={activity.id} />}
      {shown("best") && <BestPerformances activityId={activity.id} />}
    </div>
  );
}
