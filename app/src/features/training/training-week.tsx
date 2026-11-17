import { CalendarCheck } from "lucide-react";

import { localDay, parseLocalDay } from "@/features/dashboard/dashboard";
import i18n, { t } from "@/i18n";
import { cn } from "@/lib/utils";

import {
  KIND_COLORS,
  KIND_LABELS,
  VERDICT_COLORS,
  VERDICT_LABELS,
  addDays,
  checkText,
  sessionDetail,
  weekdayNames,
} from "./training-math";
import { useTrainingSessions } from "./use-training";

/**
 * The dashboard row: the sessions of the week holding `moment` and the week after, opening the
 * training page. Nothing while no plan has sessions then.
 */
export function TrainingWeek({ moment, onTraining }: { moment: Date; onTraining: () => void }) {
  const day = localDay(moment);
  const monday = addDays(day, -((moment.getDay() + 6) % 7));
  const { data: sessions = [] } = useTrainingSessions(monday, addDays(monday, 13));
  if (sessions.length === 0) return null;
  const names = weekdayNames("short");
  const dayMonth = new Intl.DateTimeFormat(i18n.language, { day: "numeric", month: "numeric" });

  return (
    <div className="flex gap-2">
      <button
        type="button"
        title={t("Training")}
        aria-label={t("Training")}
        className="flex w-10 shrink-0 items-center justify-center rounded-lg border border-dashed text-muted-foreground transition-colors hover:bg-muted/40 hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={onTraining}
      >
        <CalendarCheck className="size-4" />
      </button>
      {/* Wraps instead of scrolling sideways, whose scrollbar covered the bottom of the boxes;
          every row as tall as the tallest box. */}
      <div className="grid min-w-0 flex-1 auto-rows-fr grid-cols-[repeat(auto-fill,minmax(10rem,1fr))] gap-2">
        {sessions.map(({ planName, status }) => {
          const { session, check } = status;
          const date = parseLocalDay(session.date)!;
          const shown = check.verdict === "planned" ? null : check.verdict;
          return (
            <button
              key={session.id}
              type="button"
              title={`${planName}: ${checkText(status)}`}
              className={cn(
                "flex min-w-0 flex-col gap-0.5 rounded-lg border border-l-4 px-3 py-2 text-left text-xs transition-colors hover:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none",
                session.date === day && "bg-primary/5",
              )}
              style={{ borderLeftColor: KIND_COLORS[session.kind] }}
              onClick={onTraining}
            >
              <span className="text-muted-foreground">
                {names[(date.getDay() + 6) % 7]} {dayMonth.format(date)}
              </span>
              <span className="font-medium">{t(KIND_LABELS[session.kind])}</span>
              {/* Wrapped rather than cut off; the row takes the height of its tallest box. */}
              <span className="text-muted-foreground">{sessionDetail(session)}</span>
              {shown && <span className={VERDICT_COLORS[shown]}>{t(VERDICT_LABELS[shown])}</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
}
