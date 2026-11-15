import type { RankedEffort } from "@/bindings/RankedEffort";
import type { RouteSegment } from "@/bindings/RouteSegment";
import { t } from "@/i18n";
import { formatDate, formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

import { formatEffortSpeed } from "./segment-math";
import { RankTrophy } from "./trophy";

function Row({
  segment,
  effort,
  label,
  onOpen,
}: {
  segment: RouteSegment;
  effort: RankedEffort;
  /** Shown instead of the rank, e.g. "Latest". */
  label?: string;
  onOpen?: (activityId: number) => void;
}) {
  const content = (
    <>
      {label ? (
        <span className="w-24 shrink-0 truncate text-muted-foreground">{label}</span>
      ) : (
        <span className="flex w-24 shrink-0 items-center gap-1.5">
          <span className="w-4 text-muted-foreground tabular-nums">{effort.rank}</span>
          <RankTrophy rank={effort.rank} className="size-3.5" />
        </span>
      )}
      <span className="flex-1 truncate">{formatDate(effort.date)}</span>
      {label && (
        <span
          className="text-muted-foreground tabular-nums"
          title={t("Place {{rank}}", { rank: effort.rank })}
        >
          #{effort.rank}
        </span>
      )}
      <span className="font-semibold tabular-nums">{formatDuration(effort.elapsed)}</span>
      <span className="w-20 text-right text-muted-foreground tabular-nums">
        {formatEffortSpeed(segment.view, segment.distance, effort.elapsed)}
      </span>
    </>
  );
  const className = "flex items-center gap-2 py-1.5";
  return onOpen ? (
    <button
      type="button"
      title={t("Open details")}
      className={cn(className, "-mx-2 rounded-md px-2 text-left hover:bg-muted/50")}
      onClick={() => onOpen(effort.activityId)}
    >
      {content}
    </button>
  ) : (
    <div className={className}>{content}</div>
  );
}

/** The three fastest passes, then the latest and this year's fastest. */
export function SegmentHighlights({
  segment,
  onOpen,
  className,
}: {
  segment: RouteSegment;
  onOpen?: (activityId: number) => void;
  className?: string;
}) {
  if (segment.best.length === 0) {
    return (
      <div className={cn("py-1.5 text-xs text-muted-foreground", className)}>
        {t("No passes over this segment yet.")}
      </div>
    );
  }
  return (
    <div className={cn("grid text-xs", className)}>
      <div className="grid divide-y">
        {segment.best.map((e) => (
          <Row key={e.id} segment={segment} effort={e} onOpen={onOpen} />
        ))}
      </div>
      <div className="mt-1 grid divide-y border-t border-dashed pt-1">
        {segment.latest && (
          <Row segment={segment} effort={segment.latest} label={t("Latest")} onOpen={onOpen} />
        )}
        {segment.bestThisYear ? (
          <Row
            segment={segment}
            effort={segment.bestThisYear}
            label={t("Best this year")}
            onOpen={onOpen}
          />
        ) : (
          <div className="flex items-center gap-2 py-1.5">
            <span className="w-24 shrink-0 text-muted-foreground">{t("Best this year")}</span>
            <span className="text-muted-foreground">{t("No try this year")}</span>
          </div>
        )}
      </div>
    </div>
  );
}
