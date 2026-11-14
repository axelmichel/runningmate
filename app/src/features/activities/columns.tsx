import type { ReactNode } from "react";

import type { ActivityRow } from "@/bindings/ActivityRow";
import type { SortField } from "@/bindings/SortField";
import type { ViewMode } from "@/bindings/ViewMode";
import { formatDateTime, formatDuration, formatMinSec, formatNumber } from "@/lib/format";

export type Column = {
  header: string;
  sortField: SortField;
  align?: "right";
  /** Fixed width in px; required for columns that can be pinned (the first two of a view). */
  width?: number;
  render: (row: ActivityRow) => ReactNode;
};

type ColumnKey =
  | "type"
  | "date"
  | "title"
  | "duration"
  | "distance"
  | "elevationGain"
  | "avgSpeed"
  | "avgPace"
  | "avgHeartRate"
  | "totalSteps"
  | "avgPower"
  | "pause";

const COLUMNS: Record<ColumnKey, Column> = {
  type: {
    header: "Type",
    sortField: "type",
    width: 120,
    render: (row) => <span className="block truncate">{row.displayType}</span>,
  },
  date: {
    header: "Date",
    sortField: "date",
    width: 150,
    render: (row) => formatDateTime(row.date),
  },
  title: {
    header: "Title",
    sortField: "title",
    width: 240,
    render: (row) => (
      <span className="block truncate" title={row.title ?? undefined}>
        {row.title}
      </span>
    ),
  },
  duration: {
    header: "Duration",
    sortField: "duration",
    align: "right",
    render: (row) => formatDuration(row.duration),
  },
  distance: {
    header: "Distance (km)",
    sortField: "distance",
    align: "right",
    render: (row) => formatNumber(row.distance, 2),
  },
  elevationGain: {
    header: "Elevation (m)",
    sortField: "elevationGain",
    align: "right",
    render: (row) => formatNumber(row.elevationGain),
  },
  avgSpeed: {
    header: "Speed (km/h)",
    sortField: "avgSpeed",
    align: "right",
    render: (row) => formatNumber(row.avgSpeed, 1),
  },
  avgPace: {
    header: "Pace (/km)",
    sortField: "avgPace",
    align: "right",
    render: (row) => formatMinSec(row.avgPace),
  },
  avgHeartRate: {
    header: "HR (bpm)",
    sortField: "avgHeartRate",
    align: "right",
    render: (row) => formatNumber(row.avgHeartRate),
  },
  totalSteps: {
    header: "Steps",
    sortField: "totalSteps",
    align: "right",
    render: (row) => formatNumber(row.totalSteps),
  },
  avgPower: {
    header: "Power (W)",
    sortField: "avgPower",
    align: "right",
    render: (row) => formatNumber(row.avgPower),
  },
  pause: {
    header: "Pause",
    sortField: "pause",
    align: "right",
    render: (row) => formatMinSec(row.pause),
  },
};

const BASE: ColumnKey[] = ["date", "title", "duration", "distance", "elevationGain", "avgSpeed"];

/** Column sets per view, from `ui/table_builder.py`. */
const VIEW_COLUMNS: Record<ViewMode, ColumnKey[]> = {
  All: ["type", "date", "title", "duration", "distance"],
  Running: [...BASE, "avgPace", "avgHeartRate", "totalSteps", "pause"],
  Walking: [...BASE, "avgPace", "avgHeartRate", "totalSteps"],
  Cycling: [...BASE, "avgPace", "avgHeartRate", "avgPower", "pause"],
};

export function columnsFor(view: ViewMode): Column[] {
  return VIEW_COLUMNS[view].map((key) => COLUMNS[key]);
}
