import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect } from "react";

import type { ActivityRow } from "@/bindings/ActivityRow";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { VIEW_MODES } from "@/features/activities/view-mode";
import { useSeries } from "@/features/detail/use-series";
import { GoalStar } from "@/features/goals/goal-star";
import { TrophyBadge } from "@/features/segments/trophy";
import { useInView } from "@/hooks/use-in-view";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDateTime, formatDuration, formatMinSec, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { tileSpan } from "./dashboard";
import { RouteMap } from "./route-map";

const PAGE_SIZE = 10;
/** Photos shown per entry; the last one counts the rest. */
const PHOTOS_SHOWN = 5;
/** Up to this many photos share the row with the map. */
const MAP_ROW_PHOTOS = 2;

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="text-base font-semibold tabular-nums">{value}</div>
    </div>
  );
}

/** Map and photos of an entry, loaded once the entry is scrolled to. */
function Media({ activityId, visible }: { activityId: number; visible: boolean }) {
  const { data: points, isPending } = useSeries(activityId, visible);
  const { data: media = [], isPending: mediaPending } = useQuery({
    queryKey: ["media", activityId],
    queryFn: () => api.listMedia(activityId),
    enabled: visible,
  });
  const photos = media.filter((m) => m.mediaType === "image" && !m.missing);
  const hasMap = !!points && points.length >= 2;
  // Both first: photos arriving after the map would shrink it and show the wrong area.
  if (!visible || isPending || mediaPending) {
    return <Skeleton className="h-56 w-full" />;
  }
  if (!hasMap && photos.length === 0) return null;

  const shown = photos.slice(0, PHOTOS_SHOWN);
  const hidden = photos.length - shown.length;
  const tile = (photo: (typeof shown)[number], i: number, className: string) => (
    <div
      key={photo.id}
      className={cn("relative min-h-0 min-w-0 overflow-hidden bg-muted", className)}
    >
      <img
        src={convertFileSrc(photo.filePath)}
        alt=""
        loading="lazy"
        className="size-full object-cover"
      />
      {i === shown.length - 1 && hidden > 0 && (
        <div className="absolute inset-0 flex items-center justify-center bg-black/50 text-lg font-semibold text-white">
          +{hidden}
        </div>
      )}
    </div>
  );

  // One or two photos fit beside the map; more go below it, as a grid.
  if (hasMap && shown.length <= MAP_ROW_PHOTOS) {
    return (
      <div className="grid h-56 grid-cols-12 grid-rows-1 gap-1 overflow-hidden rounded-lg">
        <div
          className={cn(
            "overflow-hidden bg-muted",
            ["col-span-12", "col-span-8", "col-span-6"][shown.length],
          )}
        >
          <RouteMap points={points} />
        </div>
        {shown.map((photo, i) => tile(photo, i, shown.length === 1 ? "col-span-4" : "col-span-3"))}
      </div>
    );
  }
  return (
    <div className="grid gap-1 overflow-hidden rounded-lg">
      {hasMap && (
        <div className="h-56 overflow-hidden bg-muted">
          <RouteMap points={points} />
        </div>
      )}
      {shown.length > 0 && (
        <div className="grid h-80 grid-cols-4 grid-rows-2 gap-1">
          {shown.map((photo, i) => tile(photo, i, tileSpan(i, shown.length)))}
        </div>
      )}
    </div>
  );
}

function Entry({ row, onOpen }: { row: ActivityRow; onOpen: (id: number) => void }) {
  const [ref, visible] = useInView<HTMLDivElement>();
  // The row has no comment and, across all sports, no pace; the detail has both.
  const { data: detail } = useQuery({
    queryKey: ["activity", row.id],
    queryFn: () => api.getActivity(row.id),
    enabled: visible,
  });
  const Icon = VIEW_MODES.find((v) => v.mode === row.viewMode)?.icon ?? VIEW_MODES[0].icon;
  const cycling = row.viewMode === "Cycling";
  const comment = detail?.comment?.trim();
  const speed =
    detail?.avgSpeed ??
    (row.distance && row.duration ? row.distance / (row.duration / 3600) : null);

  return (
    <Card ref={ref} className="p-0">
      <button
        type="button"
        title={t("Open details")}
        className="grid gap-3 rounded-[inherit] p-4 text-left transition-colors hover:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={() => onOpen(row.id)}
      >
        <div className="flex items-start gap-3">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
            <Icon className="size-5" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-base font-semibold">{row.title || t("Activity")}</div>
            <div className="text-xs text-muted-foreground">
              {formatDateTime(row.date)} · {t(row.displayType || row.activityType || "")}
            </div>
          </div>
          <GoalStar activityId={row.id} enabled={visible} />
          <TrophyBadge activityId={row.id} enabled={visible} />
        </div>
        {comment && <p className="line-clamp-4 text-sm whitespace-pre-line">{comment}</p>}
        <div className="flex flex-wrap gap-x-8 gap-y-2">
          <Metric label={t("Distance")} value={`${formatNumber(row.distance, 2)} km`} />
          {cycling ? (
            <Metric label={t("Speed")} value={`${formatNumber(speed, 1)} km/h`} />
          ) : (
            <Metric label={t("Pace (/km)")} value={formatMinSec(detail?.avgPace)} />
          )}
          <Metric label={t("Time")} value={formatDuration(row.duration)} />
          <Metric label={t("Elevation")} value={`${formatNumber(row.elevationGain)} m`} />
        </div>
        <Media activityId={row.id} visible={visible} />
      </button>
    </Card>
  );
}

/** The latest activities of all sports, newest first, ten at a time. */
export function ActivityFeed({
  onOpen,
  onLoaded,
  until,
}: {
  onOpen: (id: number) => void;
  /** Ids of the entries loaded so far, in order (for previous/next in the detail window). */
  onLoaded: (ids: number[]) => void;
  /** Unix timestamp of the latest start shown; unset for the newest. */
  until?: number;
}) {
  const feed = useInfiniteQuery({
    queryKey: ["activities", "feed", until],
    queryFn: async ({ pageParam }) => {
      const page = await api.listActivities({
        viewMode: "All",
        offset: pageParam,
        limit: PAGE_SIZE,
        sortField: "date",
        sortDirection: "desc",
        filter: until === undefined ? {} : { maxDate: until },
      });
      return { ...page, offset: pageParam };
    },
    initialPageParam: 0,
    getNextPageParam: (last) =>
      last.offset + last.rows.length < last.total ? last.offset + last.rows.length : undefined,
  });
  const rows = feed.data?.pages.flatMap((page) => page.rows) ?? [];
  const ids = rows.map((row) => row.id).join(",");
  useEffect(() => onLoaded(ids ? ids.split(",").map(Number) : []), [ids, onLoaded]);

  if (feed.isPending) return <Skeleton className="h-96 w-full" />;
  if (rows.length === 0) {
    return <p className="text-sm text-muted-foreground">{t("No activity yet.")}</p>;
  }
  return (
    <div className="grid gap-4">
      {rows.map((row) => (
        <Entry key={row.id} row={row} onOpen={onOpen} />
      ))}
      {feed.hasNextPage && (
        <Button
          variant="outline"
          className="justify-self-center"
          disabled={feed.isFetchingNextPage}
          onClick={() => void feed.fetchNextPage()}
        >
          {t("Show more")}
        </Button>
      )}
    </div>
  );
}
