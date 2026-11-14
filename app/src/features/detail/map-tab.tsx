import "leaflet/dist/leaflet.css";

import type { LatLngBoundsExpression, LatLngTuple } from "leaflet";
import { useMemo, useState } from "react";
import { CircleMarker, MapContainer, Polyline, TileLayer, Tooltip } from "react-leaflet";

import type { TrackPoint } from "@/bindings/TrackPoint";
import type { ViewMode } from "@/bindings/ViewMode";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { formatNumber, formatPace } from "@/lib/format";

import { HEART_RATE_RAMP, PACE_RAMP, rampColor, robustRange } from "./color-scale";
import { useSeries } from "./use-series";

type Mode = "track" | "pace" | "heartRate";

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

/** Positions where each full kilometre (5 km for rides) is reached. */
function kilometreMarkers(points: TrackPoint[], every: number) {
  const markers: { km: number; position: LatLngTuple }[] = [];
  let next = every;
  for (const p of points) {
    if (p.distance >= next) {
      markers.push({ km: next, position: [p.latitude, p.longitude] });
      next += every;
    }
  }
  return markers;
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
    return <p className="text-sm text-muted-foreground">No archived TCX file for this activity.</p>;
  }
  const start = derived.line[0];
  const end = derived.line[derived.line.length - 1];

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        {(
          [
            ["track", "Track", true],
            ["pace", "Pace", derived.hasPace],
            ["heartRate", "Heart rate", derived.hasHeartRate],
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
        {mode !== "track" && <Legend mode={mode} points={points} />}
      </div>
      <div className="min-h-0 flex-1 overflow-hidden rounded-lg border">
        <MapContainer
          bounds={derived.bounds}
          boundsOptions={{ padding: [24, 24] }}
          className="h-full w-full"
        >
          <TileLayer
            attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> &copy; <a href="https://carto.com/attributions">CARTO</a>'
            url={`https://{s}.basemaps.cartocdn.com/${dark ? "dark_all" : "light_all"}/{z}/{x}/{y}{r}.png`}
          />
          {mode === "track" && (
            <Polyline
              positions={derived.line}
              pathOptions={{ color: "#701f57", weight: 4, opacity: 0.8 }}
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
          {derived.markers.map((m) => (
            <CircleMarker
              key={m.km}
              center={m.position}
              radius={9}
              pathOptions={{ color: "#fff", weight: 1.5, fillColor: "#35193e", fillOpacity: 0.9 }}
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
            <Tooltip>Start</Tooltip>
          </CircleMarker>
          <CircleMarker
            center={end}
            radius={8}
            pathOptions={{ color: "#fff", weight: 2, fillColor: "#e13342", fillOpacity: 1 }}
          >
            <Tooltip>Finish</Tooltip>
          </CircleMarker>
        </MapContainer>
      </div>
    </div>
  );
}
