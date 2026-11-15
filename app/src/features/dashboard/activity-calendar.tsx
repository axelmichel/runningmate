import type { ActivityRow } from "@/bindings/ActivityRow";
import type { PlanSession } from "@/bindings/PlanSession";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import i18n, { t } from "@/i18n";
import { formatDuration, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";
import {
  KIND_COLORS,
  KIND_LABELS,
  VERDICT_RINGS,
  checkText,
} from "@/features/training/training-math";

import { byDay, calendarWeeks, circleSize, localDay } from "./dashboard";

/** Largest circle in px; a day's circle grows with its training time. */
const MAX_CIRCLE = 36;

/** Size in px of a planned session without a run yet. */
const SESSION_CIRCLE = 20;

/** Ring colour of a day's session: its kind while ahead, how it went once done or past. */
function sessionRing(session: PlanSession): string {
  const { check } = session.status;
  return check.verdict === "planned"
    ? KIND_COLORS[session.status.session.kind]
    : VERDICT_RINGS[check.verdict];
}

function SessionLines({ sessions }: { sessions: PlanSession[] }) {
  return sessions.map(({ status }) => (
    <div key={status.session.id}>
      {t(KIND_LABELS[status.session.kind])}: {checkText(status)}
    </div>
  ));
}

/**
 * The current and the three previous weeks, one circle per training day sized by its time, as
 * Strava's training calendar. A circle opens the day's (first) activity. Days with a planned
 * session get a ring: in the session's colour while ahead, in its verdict's colour once done.
 */
export function ActivityCalendar({
  rows,
  sessions = [],
  onOpen,
  onSession,
  today = new Date(),
}: {
  rows: ActivityRow[];
  sessions?: PlanSession[];
  onOpen: (id: number) => void;
  onSession?: () => void;
  today?: Date;
}) {
  const planned = new Map<string, PlanSession[]>();
  for (const session of sessions) {
    const key = session.status.session.date;
    planned.set(key, [...(planned.get(key) ?? []), session]);
  }
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
        const daySessions = planned.get(key) ?? [];
        const ring = daySessions[0] && sessionRing(daySessions[0]);
        return (
          <div key={key} className="flex min-h-9 items-center justify-center">
            {list.length > 0 ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    aria-label={key}
                    className="rounded-full bg-primary transition-opacity hover:opacity-80 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
                    style={{
                      width: size,
                      height: size,
                      boxShadow: ring ? `0 0 0 2px var(--card), 0 0 0 4px ${ring}` : undefined,
                    }}
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
                    <SessionLines sessions={daySessions} />
                  </div>
                </TooltipContent>
              </Tooltip>
            ) : ring ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    type="button"
                    aria-label={key}
                    className={cn(
                      "rounded-full border-2 transition-opacity hover:opacity-80 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none",
                      daySessions[0].status.check.verdict === "planned" && "border-dashed",
                    )}
                    style={{ width: SESSION_CIRCLE, height: SESSION_CIRCLE, borderColor: ring }}
                    onClick={onSession}
                  />
                </TooltipTrigger>
                <TooltipContent>
                  <div className="grid gap-0.5">
                    <SessionLines sessions={daySessions} />
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
