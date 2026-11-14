import { useMemo } from "react";

import type { DayTotal } from "@/bindings/DayTotal";
import { t } from "@/i18n";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

import { heatmapWeeks } from "./heatmap";

const LEVEL_CLASSES = [
  "bg-muted",
  "bg-primary/25",
  "bg-primary/50",
  "bg-primary/75",
  "bg-primary",
] as const;

/** 52-week calendar of training time, the counterpart of `processing/plot_heatmap.py`. */
export function ActivityHeatmap({ days, end }: { days: DayTotal[]; end: Date }) {
  const weeks = useMemo(() => heatmapWeeks(days, end), [days, end]);
  return (
    <div className="overflow-x-auto">
      <div className="flex w-max gap-[3px]" role="img" aria-label={t("Training time per day")}>
        {weeks.map((week) => (
          <div key={week[0].day} className="flex flex-col gap-[3px]">
            {week.map((cell) => (
              <div
                key={cell.day}
                title={
                  cell.outside
                    ? undefined
                    : `${cell.day}: ${cell.duration ? formatDuration(cell.duration) : t("no activity")}`
                }
                className={cn(
                  "size-2.5 rounded-[2px]",
                  cell.outside ? "bg-transparent" : LEVEL_CLASSES[cell.level],
                )}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
