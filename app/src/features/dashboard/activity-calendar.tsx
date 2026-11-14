import type { ActivityRow } from "@/bindings/ActivityRow";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import i18n, { t } from "@/i18n";
import { formatDuration, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { byDay, calendarWeeks, circleSize, localDay } from "./dashboard";

/** Largest circle in px; a day's circle grows with its training time. */
const MAX_CIRCLE = 36;

/**
 * The current and the three previous weeks, one circle per training day sized by its time, as
 * Strava's training calendar. A circle opens the day's (first) activity.
 */
export function ActivityCalendar({
  rows,
  onOpen,
  today = new Date(),
}: {
  rows: ActivityRow[];
  onOpen: (id: number) => void;
  today?: Date;
}) {
  const weeks = calendarWeeks(today);
  const days = byDay(rows);
  const total = (list: ActivityRow[]) => list.reduce((sum, r) => sum + (r.duration ?? 0), 0);
  const longest = Math.max(0, ...[...days.values()].map(total));
  const todayKey = localDay(today);
  const weekday = new Intl.DateTimeFormat(i18n.language, { weekday: "narrow" });

  return (
    // The day rows share the card's height, so the calendar fills it beside the stacked boxes.
    <div className="grid h-full grid-cols-7 grid-rows-[auto_repeat(4,minmax(0,1fr))] gap-y-1 text-center">
      {weeks[0].map((day) => (
        <div key={day.getDay()} className="text-xs text-muted-foreground">
          {weekday.format(day)}
        </div>
      ))}
      {weeks.flat().map((day) => {
        const key = localDay(day);
        const list = days.get(key) ?? [];
        const seconds = total(list);
        const size = circleSize(seconds, longest) * MAX_CIRCLE;
        const future = key > todayKey;
        return (
          <div key={key} className="flex min-h-9 items-center justify-center">
            {list.length > 0 ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    aria-label={key}
                    className="rounded-full bg-primary transition-opacity hover:opacity-80 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
                    style={{ width: size, height: size }}
                    onClick={() => onOpen(list[0].id)}
                  />
                </TooltipTrigger>
                <TooltipContent>
                  <div className="grid gap-0.5">
                    {list.map((row) => (
                      <div key={row.id}>
                        {row.title || t("Activity")} · {formatNumber(row.distance, 1)} km ·{" "}
                        {formatDuration(row.duration)}
                      </div>
                    ))}
                  </div>
                </TooltipContent>
              </Tooltip>
            ) : (
              <span
                className={cn(
                  "size-1.5 rounded-full bg-muted-foreground/30",
                  future && "bg-muted-foreground/10",
                  key === todayKey && "size-2 bg-primary/50",
                )}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
