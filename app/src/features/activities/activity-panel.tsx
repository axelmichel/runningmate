import { useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Droplets, Wind } from "lucide-react";
import { useState } from "react";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import {
  formatDateTime,
  formatDuration,
  formatMinSec,
  formatNumber,
  formatPace,
} from "@/lib/format";

import { weatherLabel } from "./weather";
import { WeatherIcon } from "./weather-icon";

function Metric({ label, value, align }: { label: string; value: string; align?: "right" }) {
  return (
    <div className={align === "right" ? "text-right" : undefined}>
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
        alt="Route"
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
  const pad = 0.04;
  const points = route.points.map(([x, y]) => `${x.toFixed(4)},${y.toFixed(4)}`).join(" ");
  return (
    <svg
      role="img"
      aria-label="Route"
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
  );
}

function Overview({ activity, dataDir }: { activity: ActivityDetail; dataDir?: string }) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="leading-snug">{activity.title}</CardTitle>
        <div className="text-xs text-muted-foreground">
          {formatDateTime(activity.date)} · {activity.typeDetail || activity.activityType}
        </div>
      </CardHeader>
      <CardContent className="grid gap-4">
        <div className="grid grid-cols-2 gap-2">
          <Metric label="Duration" value={formatDuration(activity.duration)} />
          <Metric label="Pace (/km)" value={formatMinSec(activity.avgPace)} align="right" />
        </div>
        <Route activity={activity} dataDir={dataDir} />
        <div className="grid grid-cols-2 gap-2">
          <Metric label="Distance" value={`${formatNumber(activity.distance, 2)} km`} />
          <Metric
            label="Elevation"
            value={`${formatNumber(activity.elevationGain)} m`}
            align="right"
          />
        </div>
      </CardContent>
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
        <CardTitle className="text-sm">Best segments</CardTitle>
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
export function ActivityPanel({ activityId, dataDir }: { activityId?: number; dataDir?: string }) {
  const { data: activity, isPending } = useQuery({
    queryKey: ["activity", activityId],
    queryFn: () => api.getActivity(activityId!),
    enabled: activityId != null,
  });

  if (activityId == null) {
    return <p className="p-2 text-sm text-muted-foreground">No activity yet.</p>;
  }
  if (isPending || !activity) {
    return <Skeleton className="h-80 w-full" />;
  }
  return (
    <div className="grid gap-3">
      <Overview activity={activity} dataDir={dataDir} />
      <WeatherCard activityId={activity.id} />
      <BestPerformances activityId={activity.id} />
    </div>
  );
}
