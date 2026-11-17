import { ChevronLeft, ChevronRight } from "lucide-react";
import { useState } from "react";

import type { RankedEffort } from "@/bindings/RankedEffort";
import type { ViewMode } from "@/bindings/ViewMode";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { t } from "@/i18n";
import { formatDate, formatDuration, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { formatEffortSpeed } from "./segment-math";
import { RankTrophy } from "./trophy";

/** Passes per page. */
const PAGE_SIZE = 20;

/**
 * Every pass over a segment, fastest first, a page at a time; rows open the activity when
 * `onOpen` is given. Fills its parent's height and scrolls under a fixed header. Opens on the
 * page with the first pass of `highlight`.
 */
export function Leaderboard({
  efforts,
  view,
  distance,
  highlight,
  onOpen,
}: {
  efforts: RankedEffort[];
  view: ViewMode;
  /** Segment length, m. */
  distance: number;
  /** Activity whose passes are marked. */
  highlight?: number;
  onOpen?: (activityId: number) => void;
}) {
  const pageCount = Math.max(1, Math.ceil(efforts.length / PAGE_SIZE));
  const [page, setPage] = useState(() => {
    const index = efforts.findIndex((e) => e.activityId === highlight);
    return index < 0 ? 0 : Math.floor(index / PAGE_SIZE);
  });
  // Passes can go away (an activity deleted) while the dialog is open.
  const current = Math.min(page, pageCount - 1);
  const shown = efforts.slice(current * PAGE_SIZE, (current + 1) * PAGE_SIZE);
  const head = "text-right";
  const cell = "text-right tabular-nums";
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border">
        <Table containerClassName="scrollbar-visible min-h-0 flex-1 overflow-x-hidden overflow-y-auto">
          <TableHeader className="sticky top-0 z-10 bg-background">
            <TableRow>
              <TableHead className="w-16">{t("Rank")}</TableHead>
              <TableHead>{t("Date")}</TableHead>
              <TableHead>{t("Activity")}</TableHead>
              <TableHead className={head}>{t("Time")}</TableHead>
              <TableHead className={head}>
                {view === "Cycling" ? t("Speed (km/h)") : t("Pace (/km)")}
              </TableHead>
              <TableHead className={head}>{t("HR (bpm)")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {shown.map((e) => (
              <TableRow
                key={e.id}
                className={cn(
                  onOpen && "cursor-pointer",
                  e.activityId === highlight && "bg-primary/10 hover:bg-primary/15",
                )}
                title={onOpen ? t("Open details") : undefined}
                onClick={onOpen ? () => onOpen(e.activityId) : undefined}
              >
                <TableCell>
                  <span className="flex items-center gap-1.5 tabular-nums">
                    {e.rank}
                    <RankTrophy rank={e.rank} />
                  </span>
                </TableCell>
                <TableCell className="whitespace-nowrap">{formatDate(e.date)}</TableCell>
                {/* Takes the width the other columns leave and cuts long titles; cells ignore max-w. */}
                <TableCell
                  data-private
                  className="w-full max-w-0 truncate"
                  title={e.activityTitle ?? undefined}
                >
                  {e.activityTitle || t("Activity")}
                </TableCell>
                <TableCell className={cell}>{formatDuration(e.elapsed)}</TableCell>
                <TableCell className={cell}>
                  {formatEffortSpeed(view, distance, e.elapsed, false)}
                </TableCell>
                <TableCell className={cell}>{formatNumber(e.avgHeartRate)}</TableCell>
              </TableRow>
            ))}
            {efforts.length === 0 && (
              <TableRow>
                <TableCell colSpan={6} className="py-8 text-center text-muted-foreground">
                  {t("No passes over this segment yet.")}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
      {pageCount > 1 && (
        <div className="flex items-center justify-end gap-2 pt-3 text-sm text-muted-foreground">
          <span className="tabular-nums">
            {t("Page {{page}} / {{pages}} · {{from}}–{{to}} of {{total}}", {
              page: current + 1,
              pages: pageCount,
              from: current * PAGE_SIZE + 1,
              to: Math.min(efforts.length, (current + 1) * PAGE_SIZE),
              total: efforts.length,
            })}
          </span>
          <Button
            variant="outline"
            size="icon-sm"
            aria-label={t("Previous page")}
            title={t("Previous page")}
            disabled={current === 0}
            onClick={() => setPage(current - 1)}
          >
            <ChevronLeft />
          </Button>
          <Button
            variant="outline"
            size="icon-sm"
            aria-label={t("Next page")}
            title={t("Next page")}
            disabled={current + 1 >= pageCount}
            onClick={() => setPage(current + 1)}
          >
            <ChevronRight />
          </Button>
        </div>
      )}
    </div>
  );
}
