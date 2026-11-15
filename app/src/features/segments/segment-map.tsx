import "leaflet/dist/leaflet.css";

import type { LatLngTuple } from "leaflet";
import { useEffect, useMemo } from "react";
import { CircleMarker, MapContainer, Polyline, useMap } from "react-leaflet";

import type { SegmentPoint } from "@/bindings/SegmentPoint";
import { Basemap, MapCredit } from "@/features/detail/basemap";

import { boundsOf, FINISH_COLOR, SEGMENT_COLOR, START_COLOR } from "./segment-math";

/** Keeps Leaflet's size in sync when the surrounding layout changes. */
function FitToContainer() {
  const map = useMap();
  useEffect(() => {
    const observer = new ResizeObserver(() => map.invalidateSize());
    observer.observe(map.getContainer());
    return () => observer.disconnect();
  }, [map]);
  return null;
}

/** Map of a segment with start and finish, for the segment detail. */
export function SegmentMap({ points }: { points: SegmentPoint[] }) {
  const dark = document.documentElement.classList.contains("dark");
  const line = useMemo(() => points.map((p): LatLngTuple => [p.latitude, p.longitude]), [points]);
  const bounds = useMemo(() => boundsOf(line), [line]);
  if (line.length < 2) return null;
  return (
    <div className="relative isolate size-full overflow-hidden rounded-lg border">
      <MapContainer
        bounds={bounds}
        boundsOptions={{ padding: [24, 24] }}
        className="size-full"
        attributionControl={false}
      >
        <Basemap dark={dark} />
        <FitToContainer />
        <Polyline positions={line} pathOptions={{ color: SEGMENT_COLOR, weight: 5 }} />
        <CircleMarker
          center={line[0]}
          radius={7}
          pathOptions={{ color: "#fff", weight: 2, fillColor: START_COLOR, fillOpacity: 1 }}
        />
        <CircleMarker
          center={line[line.length - 1]}
          radius={7}
          pathOptions={{ color: "#fff", weight: 2, fillColor: FINISH_COLOR, fillOpacity: 1 }}
        />
      </MapContainer>
      <MapCredit />
    </div>
  );
}
