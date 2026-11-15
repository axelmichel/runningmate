import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { Route, Search } from "lucide-react";
import { useState } from "react";

import type { RouteSegment } from "@/bindings/RouteSegment";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { RouteMap } from "@/features/dashboard/route-map";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { SegmentDetailDialog } from "./segment-detail-dialog";
import { SegmentHighlights } from "./segment-highlights";
import { formatGrade, formatSegmentDistance } from "./segment-math";

const SPORT_COLORS: Record<ViewMode, string> = {
  All: "var(--primary)",
  Running: "var(--sport-running)",
  Cycling: "var(--sport-cycling)",
  Walking: "var(--sport-walking)",
};

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-sm font-semibold tabular-nums">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

/** Small card: map, numbers, the three fastest passes, the latest and this year's fastest. */
function SegmentCard({ segment, onOpen }: { segment: RouteSegment; onOpen: () => void }) {
  return (
    <Card className="overflow-hidden p-0">
      <button
        type="button"
        title={t("Open segment")}
        className="grid gap-3 rounded-[inherit] pb-3 text-left transition-colors hover:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={onOpen}
      >
        <div className="h-32 bg-muted">
          {segment.points.length >= 2 && <RouteMap points={segment.points} />}
        </div>
        <div className="grid gap-3 px-4">
          <div className="truncate font-semibold">{segment.title}</div>
          <div className="grid grid-cols-4 gap-2">
            <Metric label={t("Distance")} value={formatSegmentDistance(segment.distance)} />
            <Metric label={t("Grade")} value={formatGrade(segment.avgGrade)} />
            <Metric label={t("Climb")} value={`${formatNumber(segment.elevationGain)} m`} />
            <Metric label={t("Tries")} value={formatNumber(segment.tries)} />
          </div>
          <SegmentHighlights segment={segment} />
        </div>
      </button>
    </Card>
  );
}

function CardGrid({
  segments,
  onOpen,
}: {
  segments: RouteSegment[];
  onOpen: (id: number) => void;
}) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(18rem,1fr))] gap-4">
      {segments.map((segment) => (
        <SegmentCard key={segment.id} segment={segment} onOpen={() => onOpen(segment.id)} />
      ))}
    </div>
  );
}

/** All route segments as cards, by sport, searchable by title. */
export function SegmentsPage({ dataDir }: { dataDir?: string }) {
  const [view, setView] = useState<ViewMode>("All");
  const [search, setSearch] = useState("");
  const [segmentId, setSegmentId] = useState<number | null>(null);
  const [activityId, setActivityId] = useState<number | null>(null);
  const { data: segments, isPending } = useQuery({
    queryKey: ["routeSegments", view, search.trim()],
    queryFn: () => api.listSegments(view, search.trim()),
    placeholderData: keepPreviousData,
  });

  const groups =
    view === "All"
      ? VIEW_MODES.filter((v) => v.mode !== "All")
          .map((v) => ({ ...v, segments: (segments ?? []).filter((s) => s.view === v.mode) }))
          .filter((g) => g.segments.length > 0)
      : [];

  return (
    <div
      className={cn(
        "h-full overflow-y-auto",
        // WebKit paints Leaflet's tiles above an open dialog, whatever the z-index.
        segmentId !== null && "[&_.leaflet-container]:invisible",
      )}
    >
      <div className="grid gap-6 p-4">
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative w-64">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={search}
              placeholder={t("Search segments")}
              aria-label={t("Search segments")}
              className="pl-8"
              onChange={(event) => setSearch(event.target.value)}
            />
          </div>
          {VIEW_MODES.map(({ mode, label, icon: Icon }) => (
            <Button
              key={mode}
              size="sm"
              variant={view === mode ? "default" : "outline"}
              onClick={() => setView(mode)}
            >
              <Icon />
              {t(label)}
            </Button>
          ))}
        </div>
        {isPending ? (
          <Skeleton className="h-64 w-full" />
        ) : !segments?.length ? (
          <div className="flex flex-col items-center gap-2 py-16 text-center text-sm text-muted-foreground">
            <Route className="size-8" />
            {search.trim()
              ? t("No segment matches the search.")
              : t("No segments yet. Mark one on the map in an activity's Segments page.")}
          </div>
        ) : view === "All" ? (
          groups.map(({ mode, label, icon: Icon, segments }) => (
            <section key={mode} className="grid gap-3">
              <h2
                className="flex items-center gap-2 text-sm font-semibold"
                style={{ color: SPORT_COLORS[mode] }}
              >
                <Icon className="size-4" />
                {t(label)}
                <span className="font-normal text-muted-foreground">{segments.length}</span>
              </h2>
              <CardGrid segments={segments} onOpen={setSegmentId} />
            </section>
          ))
        ) : (
          <CardGrid segments={segments} onOpen={setSegmentId} />
        )}
      </div>
      <SegmentDetailDialog
        segmentId={segmentId}
        onClose={() => setSegmentId(null)}
        onOpenActivity={setActivityId}
        covered={activityId !== null}
      />
      <ActivityDetailDialog
        activityId={activityId}
        onClose={() => setActivityId(null)}
        dataDir={dataDir}
      />
    </div>
  );
}
