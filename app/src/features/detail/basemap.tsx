import { TileLayer } from "react-leaflet";

const ESRI = "https://server.arcgisonline.com/ArcGIS/rest/services/Canvas";
const ATTRIBUTION =
  "Tiles &copy; Esri &mdash; Esri, HERE, Garmin, &copy; OpenStreetMap contributors";

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
