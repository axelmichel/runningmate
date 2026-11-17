import "leaflet/dist/leaflet.css";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { divIcon, type LatLngTuple } from "leaflet";
import { ExternalLink, Plus, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  CircleMarker,
  MapContainer,
  Marker,
  Polyline,
  Tooltip,
  useMap,
  useMapEvents,
} from "react-leaflet";
import { toast } from "sonner";

import type { ActivitySegmentEffort } from "@/bindings/ActivitySegmentEffort";
import type { TrackPoint } from "@/bindings/TrackPoint";
import type { ViewMode } from "@/bindings/ViewMode";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Slider } from "@/components/ui/slider";
import { SegmentDetailDialog } from "@/features/segments/segment-detail-dialog";
import {
  boundsOf,
  FINISH_COLOR,
  formatEffortSpeed,
  formatGrade,
  formatSegmentDistance,
  invalidateSegments,
  MIN_SEGMENT_KM,
  selectionStats,
  START_COLOR,
} from "@/features/segments/segment-math";
import { SegmentProfile } from "@/features/segments/segment-profile";
import { RankTrophy } from "@/features/segments/trophy";
import { useSegmentEfforts } from "@/features/segments/use-segment-efforts";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDuration, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { Basemap, MapCredit } from "./basemap";
import { directionArrows, kilometreMarkers, snapToPass } from "./track-markers";
import { useSeries } from "./use-series";

const TRACK_COLOR = "#9aa3ad";
const EFFORT_COLOR = "#f37651";
const SELECTION_COLOR = "#2f80ed";

type Selection = { start: number | null; end: number | null };
const NONE: Selection = { start: null, end: null };

/** Keeps Leaflet's size in sync when the surrounding layout changes width. */
function FitToContainer() {
  const map = useMap();
  useEffect(() => {
    const observer = new ResizeObserver(() => map.invalidateSize());
    observer.observe(map.getContainer());
    return () => observer.disconnect();
  }, [map]);
  return null;
}

/** How far (screen pixels) from the track a click may be. */
const SNAP_PIXELS = 24;

/** Chevron pointing north, turned to the direction of travel. */
function arrowIcon(degrees: number) {
  return divIcon({
    className: "direction-arrow",
    html: `<svg viewBox="0 0 14 14" width="14" height="14" style="transform: rotate(${degrees}deg)"><path d="M3 10 L7 4 L11 10" fill="none" stroke="#fff" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/><path d="M3 10 L7 4 L11 10" fill="none" stroke="#35193e" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
    iconSize: [14, 14],
    iconAnchor: [7, 7],
  });
}

/** Arrows the way the activity was recorded, and the kilometres, so up and down can be told. */
function Direction({ points, view }: { points: TrackPoint[]; view: ViewMode }) {
  const arrows = useMemo(() => directionArrows(points), [points]);
  const kilometres = useMemo(
    () => kilometreMarkers(points, view === "Cycling" ? 5 : 1),
    [points, view],
  );
  return (
    <>
      {arrows.map((a, i) => (
        <Marker
          key={i}
          position={a.position}
          icon={arrowIcon(a.bearing)}
          interactive={false}
          keyboard={false}
        />
      ))}
      {kilometres.map((m) => (
        <CircleMarker
          key={m.km}
          center={m.position}
          radius={9}
          interactive={false}
          pathOptions={{ color: "#fff", weight: 1.5, fillColor: "#35193e", fillOpacity: 0.9 }}
        >
          <Tooltip permanent direction="center" className="km-label">
            {m.km}
          </Tooltip>
        </CircleMarker>
      ))}
    </>
  );
}

/**
 * Map clicks while marking: the first sets the start, the second the end, a third starts over.
 * A click only counts near the track. Where the track passes the spot more than once, the start
 * is its first pass and the end the next pass after the start.
 */
function MarkOnClick({
  points,
  selection,
  onChange,
}: {
  points: TrackPoint[];
  selection: Selection;
  onChange: (selection: Selection) => void;
}) {
  const map = useMapEvents({
    click: ({ containerPoint }) => {
      const pixels = points.map((p) =>
        map.latLngToContainerPoint([p.latitude, p.longitude]).distanceTo(containerPoint),
      );
      const startKm = selection.end === null ? selection.start : null;
      const from =
        startKm === null ? 0 : points.findIndex((p) => p.distance >= startKm + MIN_SEGMENT_KM);
      const i = from < 0 ? null : snapToPass(pixels, SNAP_PIXELS, from);
      if (i === null) {
        toast.info(t("Click closer to the track."));
        return;
      }
      onChange(
        startKm === null
          ? { start: points[i].distance, end: null }
          : { start: startKm, end: points[i].distance },
      );
    },
  });
  return null;
}

function positionsBetween(points: TrackPoint[], startKm: number, endKm: number): LatLngTuple[] {
  return points
    .filter((p) => p.distance >= startKm && p.distance <= endKm)
    .map((p): LatLngTuple => [p.latitude, p.longitude]);
}

function pointAt(points: TrackPoint[], km: number): LatLngTuple {
  const p = points.find((q) => q.distance >= km) ?? points[points.length - 1];
  return [p.latitude, p.longitude];
}

/** Marking a new segment: slider, preview numbers, profile and title. */
function NewSegmentPanel({
  activityId,
  points,
  selection,
  onChange,
  onDone,
}: {
  activityId: number;
  points: TrackPoint[];
  selection: Selection;
  onChange: (selection: Selection) => void;
  onDone: () => void;
}) {
  const queryClient = useQueryClient();
  const [title, setTitle] = useState("");
  const total = points[points.length - 1].distance;
  const { start, end } = selection;
  const complete = start !== null && end !== null;
  const stats = complete ? selectionStats(points, start, end) : null;
  const profile = useMemo(
    () =>
      complete
        ? points
            .filter((p) => p.distance >= start && p.distance <= end)
            .map((p) => ({ distance: (p.distance - start) * 1000, elevation: p.elevation }))
        : [],
    [points, start, end, complete],
  );
  const create = useMutation({
    mutationFn: () => api.createSegment(activityId, start!, end!, title),
    onSuccess: () => {
      toast.success(t("Segment saved; looking for it in your other activities"));
      invalidateSegments(queryClient);
      onDone();
    },
    onError: (error) =>
      toast.error(t("Could not save the segment: {{error}}", { error: String(error) })),
  });
  const tooShort = !stats || stats.distance < MIN_SEGMENT_KM;

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle className="text-sm">{t("New segment")}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-4">
        {!complete ? (
          <p className="text-sm text-muted-foreground">
            {start === null
              ? t("Click on the track where the segment starts.")
              : t("Now click where it ends.")}
          </p>
        ) : (
          <>
            <div className="grid gap-2">
              <Slider
                min={0}
                max={total}
                step={0.01}
                minStepsBetweenThumbs={MIN_SEGMENT_KM / 0.01}
                value={[start, end]}
                onValueChange={([a, b]) => onChange({ start: a, end: b })}
                aria-label={t("Start and end")}
              />
              <div className="flex justify-between text-xs text-muted-foreground tabular-nums">
                <span>{formatNumber(start, 2)} km</span>
                <span>{formatNumber(end, 2)} km</span>
              </div>
            </div>
            <div className="flex flex-wrap gap-x-8 gap-y-2 text-sm">
              <span>
                <span className="text-muted-foreground">{t("Distance")} </span>
                <span className="font-semibold tabular-nums">
                  {formatSegmentDistance(stats!.distance * 1000)}
                </span>
              </span>
              <span>
                <span className="text-muted-foreground">{t("Average grade")} </span>
                <span className="font-semibold tabular-nums">{formatGrade(stats!.avgGrade)}</span>
              </span>
              <span>
                <span className="text-muted-foreground">{t("Elevation gain")} </span>
                <span className="font-semibold tabular-nums">
                  {formatNumber(stats!.elevationGain)} m
                </span>
              </span>
            </div>
            <SegmentProfile points={profile} offsetKm={start} className="h-32" />
          </>
        )}
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            if (complete && !tooShort && title.trim()) create.mutate();
          }}
        >
          <Input
            value={title}
            placeholder={t("Title, e.g. Castle hill")}
            aria-label={t("Title")}
            onChange={(event) => setTitle(event.target.value)}
          />
          <Button
            type="submit"
            disabled={!complete || tooShort || !title.trim() || create.isPending}
          >
            {t("Save segment")}
          </Button>
          <Button type="button" variant="outline" onClick={onDone}>
            {t("Cancel")}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}

function EffortList({
  efforts,
  view,
  selected,
  onSelect,
  onOpen,
}: {
  efforts: ActivitySegmentEffort[];
  view: ViewMode;
  selected: number | null;
  onSelect: (id: number | null) => void;
  onOpen: (segmentId: number) => void;
}) {
  if (efforts.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("This activity passes no segment yet. Mark one with “New segment”.")}
      </p>
    );
  }
  return (
    <div className="grid divide-y rounded-lg border">
      {efforts.map((e) => (
        <div
          key={e.id}
          className={cn(
            "flex cursor-pointer items-center gap-3 px-3 py-2 text-sm transition-colors hover:bg-muted/50",
            selected === e.id && "bg-primary/10 hover:bg-primary/15",
          )}
          onClick={() => onSelect(selected === e.id ? null : e.id)}
        >
          <RankTrophy rank={e.rank} />
          <div className="min-w-0 flex-1">
            <div data-private className="truncate font-medium">
              {e.title}
            </div>
            <div className="text-xs text-muted-foreground">
              {formatSegmentDistance(e.distance)} · {formatGrade(e.avgGrade)} ·{" "}
              {formatNumber(e.elevationGain)} m
            </div>
          </div>
          <div className="text-right tabular-nums">
            <div className="font-semibold">{formatDuration(e.elapsed)}</div>
            <div className="text-xs text-muted-foreground">
              {formatEffortSpeed(view, e.distance, e.elapsed)}
            </div>
          </div>
          <div className="w-20 text-right text-xs text-muted-foreground tabular-nums">
            {t("{{rank}} of {{tries}}", { rank: e.rank, tries: e.tries })}
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("Open segment")}
            title={t("Open segment")}
            onClick={(event) => {
              event.stopPropagation();
              onOpen(e.segmentId);
            }}
          >
            <ExternalLink />
          </Button>
        </div>
      ))}
    </div>
  );
}

/** The segments an activity passes, with its place on each, and marking new ones on the map. */
export function RouteSegmentsTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const { data: points, isPending } = useSeries(activityId);
  const { data: efforts = [] } = useSegmentEfforts(activityId);
  const [marking, setMarking] = useState(false);
  const [selection, setSelection] = useState<Selection>(NONE);
  const [selected, setSelected] = useState<number | null>(null);
  const [openSegment, setOpenSegment] = useState<number | null>(null);
  const dark = document.documentElement.classList.contains("dark");

  const track = useMemo(() => {
    if (!points || points.length < 2) return null;
    const line = points.map((p): LatLngTuple => [p.latitude, p.longitude]);
    return { line, bounds: boundsOf(line) };
  }, [points]);

  if (isPending) return <Skeleton className="h-full w-full" />;
  if (!points || !track) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("No archived TCX file for this activity.")}
      </p>
    );
  }
  const stopMarking = () => {
    setMarking(false);
    setSelection(NONE);
  };
  const { start, end } = selection;

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-2">
        {marking ? (
          <Button size="sm" variant="outline" onClick={stopMarking}>
            <X />
            {t("Stop marking")}
          </Button>
        ) : (
          <Button
            size="sm"
            onClick={() => {
              setSelected(null);
              setMarking(true);
            }}
          >
            <Plus />
            {t("New segment")}
          </Button>
        )}
      </div>
      <div
        className={cn(
          "relative isolate min-h-72 flex-1 overflow-hidden rounded-lg border",
          marking && "[&_.leaflet-container]:cursor-crosshair",
          // WebKit paints Leaflet's tiles above an open dialog, whatever the z-index.
          openSegment !== null && "[&_.leaflet-container]:invisible",
        )}
      >
        <MapContainer
          bounds={track.bounds}
          boundsOptions={{ padding: [24, 24] }}
          className="h-full w-full"
          attributionControl={false}
          doubleClickZoom={!marking}
        >
          <Basemap dark={dark} />
          <FitToContainer />
          <Polyline
            positions={track.line}
            pathOptions={{ color: TRACK_COLOR, weight: 4, opacity: 0.9 }}
          />
          {!marking &&
            efforts.map((e) => (
              <Polyline
                key={e.id}
                positions={positionsBetween(points, e.startKm, e.endKm)}
                pathOptions={{
                  color: EFFORT_COLOR,
                  weight: selected === e.id ? 8 : 5,
                  opacity: selected === null || selected === e.id ? 0.95 : 0.4,
                }}
                eventHandlers={{ click: () => setSelected(e.id) }}
              />
            ))}
          {marking && start !== null && end !== null && (
            <Polyline
              positions={positionsBetween(points, start, end)}
              pathOptions={{ color: SELECTION_COLOR, weight: 6, opacity: 0.95 }}
            />
          )}
          <Direction points={points} view={view} />
          {marking && start !== null && (
            <CircleMarker
              center={pointAt(points, start)}
              radius={7}
              pathOptions={{ color: "#fff", weight: 2, fillColor: START_COLOR, fillOpacity: 1 }}
            />
          )}
          {marking && end !== null && (
            <CircleMarker
              center={pointAt(points, end)}
              radius={7}
              pathOptions={{ color: "#fff", weight: 2, fillColor: FINISH_COLOR, fillOpacity: 1 }}
            />
          )}
          {marking && <MarkOnClick points={points} selection={selection} onChange={setSelection} />}
        </MapContainer>
        <MapCredit />
      </div>
      {marking ? (
        <NewSegmentPanel
          activityId={activityId}
          points={points}
          selection={selection}
          onChange={setSelection}
          onDone={stopMarking}
        />
      ) : (
        <EffortList
          efforts={efforts}
          view={view}
          selected={selected}
          onSelect={setSelected}
          onOpen={setOpenSegment}
        />
      )}
      <SegmentDetailDialog
        segmentId={openSegment}
        onClose={() => setOpenSegment(null)}
        highlight={activityId}
      />
    </div>
  );
}
