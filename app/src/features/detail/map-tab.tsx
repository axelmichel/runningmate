import "leaflet/dist/leaflet.css";

import type { LatLngBoundsExpression, LatLngTuple } from "leaflet";
import { useQuery } from "@tanstack/react-query";
import { ChartLine, MapPin, PanelRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { CircleMarker, MapContainer, Polyline, Tooltip, useMap } from "react-leaflet";

import type { TrackPoint } from "@/bindings/TrackPoint";
import type { ViewMode } from "@/bindings/ViewMode";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber, formatPace } from "@/lib/format";

import { HEART_RATE_RAMP, PACE_RAMP, rampColor, robustRange } from "./color-scale";
import { Basemap, MapCredit } from "./basemap";
import { SplitsPanel } from "./splits-panel";
import { TrackCharts } from "./track-charts";
import { kilometreMarkers } from "./track-markers";
import { createHoverStore, indexAtKm, useHovered, type HoverStore } from "./track-position";
import { useSeries } from "./use-series";

type Mode = "track" | "pace" | "heartRate";

/** Keeps Leaflet's size in sync when the surrounding layout changes width (e.g. navigation). */
function FitToContainer() {
  const map = useMap();
  useEffect(() => {
    const observer = new ResizeObserver(() => map.invalidateSize());
    observer.observe(map.getContainer());
    return () => observer.disconnect();
  }, [map]);
  return null;
}

const MARKERS_STORAGE_KEY = "runningmate-map-distance-markers";
const SPLITS_STORAGE_KEY = "runningmate-map-splits";
/** Charts below the map; hidden until shown once. */
const CHARTS_STORAGE_KEY = "runningmate-map-charts";

/**
 * Where the charts are hovered, or else the end of the picked split. Pans the map there when a
 * split is picked and it is out of view.
 */
function PositionDot({
  points,
  store,
  pickedKm,
}: {
  points: TrackPoint[];
  store: HoverStore;
  pickedKm: number | null;
}) {
  const map = useMap();
  const hovered = useHovered(store);
  const picked = pickedKm === null ? null : indexAtKm(points, pickedKm);
  useEffect(() => {
    if (picked === null) return;
    const p = points[picked];
    if (!map.getBounds().contains([p.latitude, p.longitude])) {
      map.panTo([p.latitude, p.longitude]);
    }
  }, [map, points, picked]);
  const index = hovered ?? picked;
  if (index === null) return null;
  const p = points[index];
  return (
    <CircleMarker
      center={[p.latitude, p.longitude]}
      radius={7}
      interactive={false}
      pathOptions={{ color: "#fff", weight: 3, fillColor: "#2f80ed", fillOpacity: 1 }}
    />
  );
}

/** Number of colour steps; consecutive points in the same step share one polyline. */
const COLOR_STEPS = 12;

type Run = { positions: LatLngTuple[]; color: string };

function colouredRuns(points: TrackPoint[], mode: Exclude<Mode, "track">): Run[] {
  const value = (p: TrackPoint) => (mode === "pace" ? p.pace : p.heartRate);
  const [low, high] = robustRange(points.map((p) => value(p) ?? Number.NaN));
  const ramp = mode === "pace" ? PACE_RAMP : HEART_RATE_RAMP;
  const runs: Run[] = [];
  let lastStep = -1;
  points.forEach((p, i) => {
    const v = value(p);
    const step = v == null ? lastStep : Math.round(((v - low) / (high - low)) * COLOR_STEPS);
    const position: LatLngTuple = [p.latitude, p.longitude];
    if (step !== lastStep || runs.length === 0) {
      // Start the new run at the previous point so the line has no gaps.
      const previous = i > 0 ? points[i - 1] : p;
      runs.push({
        positions: [[previous.latitude, previous.longitude], position],
        color: rampColor(ramp, step / COLOR_STEPS),
      });
      lastStep = step;
    } else {
      runs[runs.length - 1].positions.push(position);
    }
  });
  return runs;
}

function Legend({ mode, points }: { mode: Exclude<Mode, "track">; points: TrackPoint[] }) {
  const value = (p: TrackPoint) => (mode === "pace" ? p.pace : p.heartRate);
  const [low, high] = robustRange(points.map((p) => value(p) ?? Number.NaN));
  const ramp = mode === "pace" ? PACE_RAMP : HEART_RATE_RAMP;
  const label = (v: number) =>
    mode === "pace" ? `${formatPace(v)} /km` : `${formatNumber(v)} bpm`;
  const gradient = Array.from({ length: 6 }, (_, i) => rampColor(ramp, i / 5)).join(", ");
  return (
    <div className="flex items-center gap-2 text-xs text-muted-foreground">
      <span>{label(low)}</span>
      <span
        className="h-2 w-32 rounded-full"
        style={{ background: `linear-gradient(to right, ${gradient})` }}
      />
      <span>{label(high)}</span>
    </div>
  );
}

/** Route map with track, pace or heart-rate colouring (after `processing/track_map.py`). */
export function MapTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const { data: points, isPending } = useSeries(activityId);
  const [mode, setMode] = useState<Mode>("track");
  const [showMarkers, setShowMarkers] = useState(
    () => localStorage.getItem(MARKERS_STORAGE_KEY) !== "hidden",
  );
  const toggleMarkers = () =>
    setShowMarkers((shown) => {
      localStorage.setItem(MARKERS_STORAGE_KEY, shown ? "hidden" : "shown");
      return !shown;
    });
  const [showSplits, setShowSplits] = useState(
    () => localStorage.getItem(SPLITS_STORAGE_KEY) !== "hidden",
  );
  const toggleSplits = () =>
    setShowSplits((shown) => {
      localStorage.setItem(SPLITS_STORAGE_KEY, shown ? "hidden" : "shown");
      return !shown;
    });
  const [showCharts, setShowCharts] = useState(
    () => localStorage.getItem(CHARTS_STORAGE_KEY) === "shown",
  );
  const toggleCharts = () =>
    setShowCharts((shown) => {
      localStorage.setItem(CHARTS_STORAGE_KEY, shown ? "hidden" : "shown");
      return !shown;
    });
  const { data: splits = [] } = useQuery({
    queryKey: ["segments", activityId],
    queryFn: () => api.activitySegments(activityId),
  });
  const [picked, setPicked] = useState<{ index: number; km: number } | null>(null);
  const [hover] = useState(createHoverStore);
  const dark = document.documentElement.classList.contains("dark");

  const derived = useMemo(() => {
    if (!points || points.length < 2) return null;
    const lats = points.map((p) => p.latitude);
    const lons = points.map((p) => p.longitude);
    const bounds: LatLngBoundsExpression = [
      [Math.min(...lats), Math.min(...lons)],
      [Math.max(...lats), Math.max(...lons)],
    ];
    return {
      bounds,
      line: points.map((p): LatLngTuple => [p.latitude, p.longitude]),
      markers: kilometreMarkers(points, view === "Cycling" ? 5 : 1),
      pace: colouredRuns(points, "pace"),
      heartRate: colouredRuns(points, "heartRate"),
      hasPace: points.some((p) => p.pace != null),
      hasHeartRate: points.some((p) => p.heartRate != null),
    };
  }, [points, view]);

  if (isPending) return <Skeleton className="h-full w-full" />;
  if (!points || !derived) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("No archived TCX file for this activity.")}
      </p>
    );
  }
  const start = derived.line[0];
  const end = derived.line[derived.line.length - 1];

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        {(
          [
            ["track", t("Track"), true],
            ["pace", t("Pace"), derived.hasPace],
            ["heartRate", t("Heart rate"), derived.hasHeartRate],
          ] as const
        ).map(([value, label, enabled]) => (
          <Button
            key={value}
            size="sm"
            variant={mode === value ? "default" : "outline"}
            disabled={!enabled}
            onClick={() => setMode(value)}
          >
            {label}
          </Button>
        ))}
        <Button
          size="sm"
          variant={showMarkers ? "secondary" : "ghost"}
          aria-pressed={showMarkers}
          title={showMarkers ? t("Hide distance markers") : t("Show distance markers")}
          onClick={toggleMarkers}
        >
          <MapPin />
          {view === "Cycling" ? t("5 km markers") : t("Km markers")}
        </Button>
        {mode !== "track" && <Legend mode={mode} points={points} />}
        <Button
          size="sm"
          variant={showSplits ? "secondary" : "ghost"}
          className="ml-auto"
          aria-pressed={showSplits}
          title={showSplits ? t("Hide splits") : t("Show splits")}
          onClick={toggleSplits}
        >
          <PanelRight />
          {t("Splits")}
        </Button>
        <Button
          size="sm"
          variant={showCharts ? "secondary" : "ghost"}
          aria-pressed={showCharts}
          title={showCharts ? t("Hide charts") : t("Show charts")}
          onClick={toggleCharts}
        >
          <ChartLine />
          {t("Charts")}
        </Button>
      </div>
      <div className="flex min-h-72 flex-1 gap-3">
        <div className="relative isolate min-w-0 flex-1 overflow-hidden rounded-lg border">
          <MapContainer
            bounds={derived.bounds}
            boundsOptions={{ padding: [24, 24] }}
            className="h-full w-full"
            attributionControl={false}
          >
            <Basemap dark={dark} />
            <FitToContainer />
            {mode === "track" && (
              <Polyline
                positions={derived.line}
                pathOptions={{ color: "#f37651", weight: 4, opacity: 0.9 }}
              />
            )}
            {mode !== "track" &&
              derived[mode].map((run, i) => (
                <Polyline
                  key={i}
                  positions={run.positions}
                  pathOptions={{ color: run.color, weight: 5, opacity: 0.9 }}
                />
              ))}
            {showMarkers &&
              derived.markers.map((m) => (
                <CircleMarker
                  key={m.km}
                  center={m.position}
                  radius={9}
                  pathOptions={{
                    color: "#fff",
                    weight: 1.5,
                    fillColor: "#35193e",
                    fillOpacity: 0.9,
                  }}
                >
                  <Tooltip permanent direction="center" className="km-label">
                    {m.km}
                  </Tooltip>
                </CircleMarker>
              ))}
            <CircleMarker
              center={start}
              radius={8}
              pathOptions={{ color: "#fff", weight: 2, fillColor: "#f6b48f", fillOpacity: 1 }}
            >
              <Tooltip>{t("Start")}</Tooltip>
            </CircleMarker>
            <CircleMarker
              center={end}
              radius={8}
              pathOptions={{ color: "#fff", weight: 2, fillColor: "#e13342", fillOpacity: 1 }}
            >
              <Tooltip>{t("Finish")}</Tooltip>
            </CircleMarker>
            <PositionDot points={points} store={hover} pickedKm={picked?.km ?? null} />
          </MapContainer>
          <MapCredit />
        </div>
        {showSplits && (
          <SplitsPanel
            rows={splits}
            selected={picked?.index ?? null}
            onSelect={(index, km) => setPicked(index === null ? null : { index, km })}
          />
        )}
      </div>
      {showCharts && (
        <div className="shrink-0">
          <TrackCharts points={points} markKm={picked?.km ?? null} onHover={hover.set} />
        </div>
      )}
    </div>
  );
}
