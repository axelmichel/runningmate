import type { SegmentRow } from "@/bindings/SegmentRow";
import { t } from "@/i18n";
import { formatDuration, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { splitEnds } from "./track-position";

/** The splits beside the map; picking one moves the position dot to where it ends. */
export function SplitsPanel({
  rows,
  selected,
  onSelect,
}: {
  rows: SegmentRow[];
  /** Index of the picked split. */
  selected: number | null;
  onSelect: (index: number | null, endKm: number) => void;
}) {
  const ends = splitEnds(rows);
  return (
    <aside className="flex w-60 shrink-0 flex-col overflow-hidden rounded-lg border">
      <div className="grid grid-cols-[1fr_auto_auto] gap-3 border-b px-3 py-2 text-xs font-medium text-muted-foreground">
        <span>{t("Distance (km)")}</span>
        <span className="w-14 text-right">{t("Time")}</span>
        <span className="w-14 text-right">{t("km/h")}</span>
      </div>
      <div className="scrollbar-visible min-h-0 flex-1 overflow-y-auto">
        {rows.map((row, i) => {
          const km = Math.round(ends[i] * 100) / 100;
          return (
            <button
              key={row.index}
              type="button"
              className={cn(
                "grid w-full grid-cols-[1fr_auto_auto] gap-3 px-3 py-1.5 text-left text-sm tabular-nums transition-colors hover:bg-muted/50",
                selected === i && "bg-primary/10 hover:bg-primary/15",
              )}
              aria-pressed={selected === i}
              onClick={() => onSelect(selected === i ? null : i, ends[i])}
            >
              <span>{formatNumber(km, Number.isInteger(km) ? 0 : 2)}</span>
              <span className="w-14 text-right">{formatDuration(row.duration)}</span>
              <span className="w-14 text-right">
                {formatNumber(row.speed == null ? null : row.speed * 3.6, 1)}
              </span>
            </button>
          );
        })}
        {rows.length === 0 && (
          <p className="px-3 py-4 text-sm text-muted-foreground">
            {t("No splits for this activity.")}
          </p>
        )}
      </div>
    </aside>
  );
}
