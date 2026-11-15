import { ChevronLeft, ChevronRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { t } from "@/i18n";
import { formatDate } from "@/lib/format";

import {
  dashboardMoment,
  localDay,
  parseLocalDay,
  periodStart,
  pickDay,
  stepDay,
} from "./dashboard";

/**
 * Moves the dashboard back and forward in time, four weeks a step, or to a picked day. `day` is
 * the last day shown, `null` for today.
 */
export function PeriodNav({
  day,
  today,
  onChange,
}: {
  day: Date | null;
  today: Date;
  onChange: (day: Date | null) => void;
}) {
  const moment = dashboardMoment(day, today);
  const from = periodStart(moment) + 1;

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button
        variant="outline"
        size="icon-sm"
        title={t("4 weeks back")}
        aria-label={t("4 weeks back")}
        onClick={() => onChange(stepDay(day, -1, today))}
      >
        <ChevronLeft />
      </Button>
      <Input
        type="date"
        className="h-7 w-auto"
        value={localDay(moment)}
        max={localDay(today)}
        title={t("Show the 4 weeks up to this day")}
        aria-label={t("Show the 4 weeks up to this day")}
        onChange={(event) => {
          const picked = parseLocalDay(event.target.value);
          if (picked) onChange(pickDay(picked, today));
        }}
      />
      <Button
        variant="outline"
        size="icon-sm"
        title={t("4 weeks forward")}
        aria-label={t("4 weeks forward")}
        disabled={day === null}
        onClick={() => onChange(stepDay(day, 1, today))}
      >
        <ChevronRight />
      </Button>
      <span className="text-sm text-muted-foreground tabular-nums">
        {formatDate(from)} – {formatDate(Math.floor(moment.getTime() / 1000))}
      </span>
      {day && (
        <Button variant="ghost" size="sm" onClick={() => onChange(null)}>
          {t("Back to today")}
        </Button>
      )}
    </div>
  );
}
