import type { ActivityDetail } from "@/bindings/ActivityDetail";
import type { TrackPoint } from "@/bindings/TrackPoint";
import { t, tk } from "@/i18n";
import { formatDateTime, formatDuration, formatMinSec, formatNumber } from "@/lib/format";

/** Output sizes in pixels, after the platforms' recommended upload sizes. */
export const SIZE_PRESETS = [
  { id: "instagram-square", label: tk("Instagram post (1:1)"), width: 1080, height: 1080 },
  { id: "instagram-portrait", label: tk("Instagram portrait (4:5)"), width: 1080, height: 1350 },
  { id: "story", label: tk("Instagram / Facebook story (9:16)"), width: 1080, height: 1920 },
  { id: "x", label: tk("X post (16:9)"), width: 1600, height: 900 },
  { id: "facebook", label: tk("Facebook post"), width: 1200, height: 630 },
] as const;

export const MIN_SIZE = 200;
export const MAX_SIZE = 4096;

export type BackgroundKind = "transparent" | "color" | "gradient" | "image" | "map";

export const STAT_KEYS = [
  "title",
  "date",
  "distance",
  "duration",
  "pace",
  "elevation",
  "heartRate",
  "calories",
] as const;
export type StatKey = (typeof STAT_KEYS)[number];

/** Centre of an element on the image, 0–1 of width and height. */
export type Point = { x: number; y: number };

/** An element placed on the image with a size factor. */
export type Placement = Point & { size: number };

/** What can be dragged: the route or one of the facts. */
export type ElementId = "route" | StatKey;

/** Where each fact starts: title and date at the top, the figures in two rows at the bottom. */
export const DEFAULT_POSITIONS: Record<StatKey, Point> = {
  title: { x: 0.5, y: 0.08 },
  date: { x: 0.5, y: 0.135 },
  distance: { x: 0.2, y: 0.8 },
  duration: { x: 0.5, y: 0.8 },
  pace: { x: 0.8, y: 0.8 },
  elevation: { x: 0.2, y: 0.91 },
  heartRate: { x: 0.5, y: 0.91 },
  calories: { x: 0.8, y: 0.91 },
};

export type Scene = {
  width: number;
  height: number;
  background: BackgroundKind;
  color: string;
  gradientFrom: string;
  gradientTo: string;
  /** Degrees, as in CSS: 0 runs bottom to top, 90 left to right. */
  gradientAngle: number;
  /** Darkens a photo or map background, 0–1. */
  dim: number;
  mapDark: boolean;
  route: Placement & { show: boolean; color: string; lineWidth: number };
  /** The facts, each placed on its own; `size` scales all their text. */
  info: {
    show: boolean;
    color: string;
    size: number;
    fields: StatKey[];
    positions: Record<StatKey, Point>;
  };
  /** Elements that are selected, moved and arranged together. */
  groups: ElementId[][];
  /** Editing aid, never part of the image: dragged elements snap to it while it's shown. */
  grid: { show: boolean; divisions: number };
};

export const DEFAULT_SCENE: Scene = {
  width: 1080,
  height: 1350,
  background: "gradient",
  color: "#35193e",
  gradientFrom: "#35193e",
  gradientTo: "#e13342",
  gradientAngle: 160,
  dim: 0.25,
  mapDark: true,
  route: { show: true, x: 0.5, y: 0.45, size: 0.6, color: "#ffffff", lineWidth: 1 },
  info: {
    show: true,
    size: 1,
    color: "#ffffff",
    fields: ["title", "date", "distance", "duration", "pace"],
    positions: DEFAULT_POSITIONS,
  },
  groups: [],
  grid: { show: false, divisions: 12 },
};

/** Grid sizes offered: squares per shorter side. */
export const GRID_DIVISIONS = [4, 6, 8, 12, 16, 24] as const;

/** Side of a grid square in pixels: the image's shorter side split into `divisions`. */
export function gridStep(scene: Scene): number {
  return Math.min(scene.width, scene.height) / scene.grid.divisions;
}

/**
 * Grid lines along one side of `length` pixels, `step` apart. They run through the centre,
 * so an element snapped there sits exactly in the middle.
 */
export function gridLines(length: number, step: number): number[] {
  const first = length / 2 - Math.floor(length / 2 / step) * step;
  const lines: number[] = [];
  for (let at = first; at <= length + 1e-6; at += step) lines.push(at);
  return lines;
}

/** The grid line nearest to `value` (pixels) on a side of `length`. */
export function snapToGrid(value: number, length: number, step: number): number {
  return length / 2 + Math.round((value - length / 2) / step) * step;
}

/** Web Mercator in world units: x and y from 0 to 1, y pointing down (as map tiles). */
export function mercator(latitude: number, longitude: number): [number, number] {
  const lat = (Math.max(-85, Math.min(85, latitude)) * Math.PI) / 180;
  return [(longitude + 180) / 360, (1 - Math.log(Math.tan(lat) + 1 / Math.cos(lat)) / Math.PI) / 2];
}

export type Route = {
  points: [number, number][];
  /** Bounding box in world units: min x, min y, max x, max y. */
  box: [number, number, number, number];
};

export function routeOf(track: Pick<TrackPoint, "latitude" | "longitude">[]): Route | null {
  const points = track.map((p) => mercator(p.latitude, p.longitude));
  if (points.length < 2) return null;
  const box: Route["box"] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const [x, y] of points) {
    box[0] = Math.min(box[0], x);
    box[1] = Math.min(box[1], y);
    box[2] = Math.max(box[2], x);
    box[3] = Math.max(box[3], y);
  }
  return { points, box };
}

/** World → canvas: `canvas = world × scale + offset`. */
export type Transform = { scale: number; offsetX: number; offsetY: number };

/**
 * Fits the route's longer side to `size` × the image's shorter side, centred on its
 * placement. A map background uses the same transform, so the route stays on its streets.
 */
export function routeTransform(route: Route, scene: Scene): Transform {
  const [x0, y0, x1, y1] = route.box;
  const side = scene.route.size * Math.min(scene.width, scene.height);
  const scale = side / Math.max(x1 - x0, y1 - y0, 1e-9);
  return {
    scale,
    offsetX: scene.route.x * scene.width - ((x0 + x1) / 2) * scale,
    offsetY: scene.route.y * scene.height - ((y0 + y1) / 2) * scale,
  };
}

/** Highest zoom the tile server has. */
export const MAX_TILE_ZOOM = 16;

export type Tile = {
  z: number;
  x: number;
  y: number;
  /** Top-left corner and side on the canvas, in pixels. */
  left: number;
  top: number;
  side: number;
};

/** Map tiles covering a `width × height` canvas, at the zoom closest to 256 px per tile. */
export function tilesFor(transform: Transform, width: number, height: number): Tile[] {
  const z = Math.max(0, Math.min(MAX_TILE_ZOOM, Math.round(Math.log2(transform.scale / 256))));
  const count = 2 ** z;
  const side = transform.scale / count;
  const tile = (canvas: number, offset: number) => Math.floor((canvas - offset) / side);
  const tiles: Tile[] = [];
  for (
    let ty = Math.max(0, tile(0, transform.offsetY));
    ty <= tile(height, transform.offsetY);
    ty++
  ) {
    if (ty >= count) break;
    for (let tx = tile(0, transform.offsetX); tx <= tile(width, transform.offsetX); tx++) {
      tiles.push({
        z,
        // East and west of the date line repeat the world.
        x: ((tx % count) + count) % count,
        y: ty,
        left: tx * side + transform.offsetX,
        top: ty * side + transform.offsetY,
        side,
      });
    }
  }
  return tiles;
}

export const STAT_LABELS: Record<StatKey, string> = {
  title: tk("Title"),
  date: tk("Date"),
  distance: tk("Distance"),
  duration: tk("Duration"),
  pace: tk("Pace"),
  elevation: tk("Elevation"),
  heartRate: tk("Heart rate"),
  calories: tk("Calories"),
};

/** One fact as text: the title and date are single lines, the figures a value with a label. */
export type InfoItem = { key: StatKey; value: string; label: string | null };

/** The chosen facts of an activity as text; facts the activity doesn't have are left out. */
export function infoItems(activity: ActivityDetail, fields: StatKey[]): InfoItem[] {
  const cycling = activity.viewMode === "Cycling";
  const values: Record<StatKey, string | null> = {
    title: activity.title || null,
    date: activity.date != null ? formatDateTime(activity.date) : null,
    distance: activity.distance ? `${formatNumber(activity.distance, 2)} km` : null,
    duration: activity.duration ? formatDuration(activity.duration).replace(/^0:/, "") : null,
    pace: cycling
      ? activity.avgSpeed
        ? `${formatNumber(activity.avgSpeed, 1)} km/h`
        : null
      : activity.avgPace
        ? `${formatMinSec(activity.avgPace)} /km`
        : null,
    elevation: activity.elevationGain ? `${formatNumber(activity.elevationGain)} m` : null,
    heartRate: activity.avgHeartRate ? `${activity.avgHeartRate} bpm` : null,
    calories: activity.calories ? `${formatNumber(activity.calories)} kcal` : null,
  };
  return STAT_KEYS.filter((key) => fields.includes(key) && values[key]).map((key) => ({
    key,
    value: values[key]!,
    label:
      key === "title" || key === "date"
        ? null
        : key === "pace" && cycling
          ? t("Average speed")
          : t(STAT_LABELS[key]),
  }));
}

/** File name for the image, from the activity's title and date. */
export function shareFileName(activity: ActivityDetail): string {
  const day =
    activity.date != null ? new Date(activity.date * 1000).toISOString().slice(0, 10) : "";
  return [activity.title || "RunningMate", day].filter(Boolean).join(" ");
}
