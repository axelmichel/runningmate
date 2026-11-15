import "leaflet/dist/leaflet.css";

import { keepPreviousData, useQuery } from "@tanstack/react-query";
import L, { type LatLngBoundsExpression, type LatLngTuple } from "leaflet";
import { Flame, Loader2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { MapContainer, useMap } from "react-leaflet";

import type { ActivityFilter } from "@/bindings/ActivityFilter";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { OPEN_RANGE, toDateRange, type RangeInput } from "@/features/connectors/range";
import { RangeFields } from "@/features/connectors/range-fields";
import { Basemap, MapCredit } from "@/features/detail/basemap";
import { t } from "@/i18n";
import { api } from "@/lib/api";

import { decodePolyline } from "./polyline";

/** Where no track has been recorded yet: central Europe. */
const EMPTY_BOUNDS: LatLngBoundsExpression = [
  [45, 0],
  [55, 20],
];

function boundsOf(tracks: LatLngTuple[][]): LatLngBoundsExpression | null {
  let [south, west, north, east] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const track of tracks) {
    for (const [lat, lon] of track) {
      south = Math.min(south, lat);
      north = Math.max(north, lat);
      west = Math.min(west, lon);
      east = Math.max(east, lon);
    }
  }
  return south === Infinity
    ? null
    : [
        [south, west],
        [north, east],
      ];
}

/**
 * Every track as a thin, translucent line on one shared canvas, so routes run often add up to
 * a brighter line. Drawn with Leaflet directly: thousands of react-leaflet layers would be slow.
 */
function TrackLayer({ tracks, dark }: { tracks: LatLngTuple[][]; dark: boolean }) {
  const map = useMap();
  useEffect(() => {
    const renderer = L.canvas({ padding: 0.5 });
    const style = { renderer, color: "#f37651", weight: 2, opacity: dark ? 0.45 : 0.35 };
    const layer = L.layerGroup(tracks.map((track) => L.polyline(track, style))).addTo(map);
    return () => {
      layer.remove();
      renderer.remove();
    };
  }, [map, tracks, dark]);
  return null;
}

/** Fits the tracks whenever the selection changes, and keeps the map sized to its container. */
function FitTracks({ bounds }: { bounds: LatLngBoundsExpression | null }) {
  const map = useMap();
  useEffect(() => {
    const observer = new ResizeObserver(() => map.invalidateSize({ animate: false }));
    observer.observe(map.getContainer());
    return () => observer.disconnect();
  }, [map]);
  useEffect(() => {
    if (bounds) map.fitBounds(bounds, { padding: [24, 24], animate: false });
  }, [map, bounds]);
  return null;
}

/** All GPS tracks on one map, by sport and period. */
export function RouteHeatmapPage() {
  const [view, setView] = useState<ViewMode>("All");
  const [range, setRange] = useState<RangeInput>(OPEN_RANGE);
  const filter = useMemo((): ActivityFilter => {
    const { from, to } = toDateRange(range);
    return { minDate: from, maxDate: to };
  }, [range]);
  // Tracks are stored once per activity, so a result stays valid until activities change;
  // imports, edits and deletes invalidate the "activities" key.
  const { data, isPending, isFetching } = useQuery({
    queryKey: ["activities", "routeHeatmap", view, filter],
    queryFn: () => api.routeHeatmap(view, filter),
    staleTime: Infinity,
    placeholderData: keepPreviousData,
  });
  const tracks = useMemo(() => (data?.tracks ?? []).map(decodePolyline), [data]);
  const bounds = useMemo(() => boundsOf(tracks), [tracks]);
  const dark = document.documentElement.classList.contains("dark");

  return (
    <div className="flex h-full flex-col gap-4 p-4">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div className="flex flex-wrap items-end gap-4">
          <div className="flex flex-wrap gap-2 pb-6">
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
          <RangeFields
            id="heatmap-range"
            value={range}
            onChange={setRange}
            hint={t("Optional. Without dates, every activity is shown.")}
          />
        </div>
        <p className="flex items-center gap-2 pb-6 text-sm text-muted-foreground">
          {isFetching && <Loader2 className="size-4 animate-spin" />}
          {data && t("{{count}} activities", { count: tracks.length })}
        </p>
      </div>
      {isPending ? (
        <div className="relative min-h-0 flex-1">
          <Skeleton className="size-full" />
          <p className="absolute inset-0 flex items-center justify-center text-sm text-muted-foreground">
            {t("Preparing the tracks. The first time this can take a moment.")}
          </p>
        </div>
      ) : (
        <div className="relative isolate min-h-0 flex-1 overflow-hidden rounded-lg border">
          <MapContainer
            bounds={bounds ?? EMPTY_BOUNDS}
            className="size-full"
            attributionControl={false}
            preferCanvas
          >
            <Basemap dark={dark} />
            <TrackLayer tracks={tracks} dark={dark} />
            <FitTracks bounds={bounds} />
          </MapContainer>
          <MapCredit />
          {tracks.length === 0 && (
            <div className="pointer-events-none absolute inset-0 z-[1000] flex flex-col items-center justify-center gap-2 text-sm text-muted-foreground">
              <Flame className="size-8" />
              <span className="rounded-md bg-background/80 px-3 py-1">
                {t("No activities with a GPS track in this selection.")}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
