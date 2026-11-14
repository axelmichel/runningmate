import "leaflet/dist/leaflet.css";

import type { LatLngBoundsExpression, LatLngTuple } from "leaflet";
import { useEffect, useMemo } from "react";
import { CircleMarker, MapContainer, Polyline, useMap } from "react-leaflet";

import type { TrackPoint } from "@/bindings/TrackPoint";
import { Basemap, MapCredit } from "@/features/detail/basemap";

/**
 * Fits the route again whenever the map's size changes: the map is created before the feed's
 * layout settles, and a fit at the wrong size shows the wrong area.
 */
function FitRoute({ bounds }: { bounds: LatLngBoundsExpression }) {
  const map = useMap();
  useEffect(() => {
    const observer = new ResizeObserver(() => {
      map.invalidateSize({ animate: false });
      map.fitBounds(bounds, { padding: [16, 16], animate: false });
    });
    observer.observe(map.getContainer());
    return () => observer.disconnect();
  }, [map, bounds]);
  return null;
}

/**
 * A still map of the route for the feed: no panning, zooming or controls. Isolated, so
 * Leaflet's pane z-indices (400 and up) stay inside it instead of covering dialogs.
 */
export function RouteMap({ points }: { points: TrackPoint[] }) {
  const dark = document.documentElement.classList.contains("dark");
  const route = useMemo(() => {
    const line = points.map((p): LatLngTuple => [p.latitude, p.longitude]);
    const lats = line.map(([lat]) => lat);
    const lons = line.map(([, lon]) => lon);
    const bounds: LatLngBoundsExpression = [
      [Math.min(...lats), Math.min(...lons)],
      [Math.max(...lats), Math.max(...lons)],
    ];
    return { line, bounds };
  }, [points]);
  const end = route.line[route.line.length - 1];

  return (
    <div className="pointer-events-none relative isolate size-full">
      <MapContainer
        bounds={route.bounds}
        boundsOptions={{ padding: [16, 16] }}
        className="size-full"
        zoomControl={false}
        attributionControl={false}
        dragging={false}
        scrollWheelZoom={false}
        doubleClickZoom={false}
        touchZoom={false}
        boxZoom={false}
        keyboard={false}
      >
        <Basemap dark={dark} />
        <FitRoute bounds={route.bounds} />
        <Polyline
          positions={route.line}
          pathOptions={{ color: "#f37651", weight: 3.5, opacity: 0.95 }}
        />
        <CircleMarker
          center={route.line[0]}
          radius={5}
          pathOptions={{ color: "#fff", weight: 1.5, fillColor: "#f6b48f", fillOpacity: 1 }}
        />
        <CircleMarker
          center={end}
          radius={5}
          pathOptions={{ color: "#fff", weight: 1.5, fillColor: "#e13342", fillOpacity: 1 }}
        />
      </MapContainer>
      <MapCredit />
    </div>
  );
}
