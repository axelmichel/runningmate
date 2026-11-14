import { Info } from "lucide-react";
import { TileLayer } from "react-leaflet";

const ESRI = "https://server.arcgisonline.com/ArcGIS/rest/services/Canvas";
const ATTRIBUTION =
  "Tiles &copy; Esri &mdash; Esri, HERE, Garmin, &copy; OpenStreetMap contributors";
/** The same credit as plain text, for `MapCredit`. */
const ATTRIBUTION_TEXT =
  "Leaflet | Tiles © Esri — Esri, HERE, Garmin, © OpenStreetMap contributors";

/**
 * Esri's light or dark gray canvas with a separate label layer. These raster tiles need no API
 * key; CARTO's basemaps now do, and OpenStreetMap blocks app traffic on its own tile servers.
 */
export function Basemap({ dark }: { dark: boolean }) {
  const theme = dark ? "Dark" : "Light";
  return (
    <>
      <TileLayer
        key={`base-${theme}`}
        attribution={ATTRIBUTION}
        url={`${ESRI}/World_${theme}_Gray_Base/MapServer/tile/{z}/{y}/{x}`}
        maxNativeZoom={16}
        maxZoom={19}
      />
      <TileLayer
        key={`labels-${theme}`}
        url={`${ESRI}/World_${theme}_Gray_Reference/MapServer/tile/{z}/{y}/{x}`}
        maxNativeZoom={16}
        maxZoom={19}
      />
    </>
  );
}

/**
 * The tile and data credit the Esri and OpenStreetMap terms ask for, as a small ⓘ in the map's
 * bottom-right corner that shows the full text on hover. Use with `attributionControl={false}`
 * on the `MapContainer`, inside a `relative` parent. Above Leaflet's panes (z-index 400–700).
 */
export function MapCredit() {
  return (
    <span className="group pointer-events-auto absolute right-1 bottom-1 z-[1000] flex items-center gap-1 rounded-full bg-black/55 text-[10px] text-white/90">
      <span className="hidden pl-2 whitespace-nowrap group-hover:inline">{ATTRIBUTION_TEXT}</span>
      <Info className="size-4 shrink-0 p-0.5" aria-label={ATTRIBUTION_TEXT} />
    </span>
  );
}
