import { Trophy } from "lucide-react";

import { t } from "@/i18n";
import { cn } from "@/lib/utils";

import { TROPHY_COLORS, TROPHY_RANKS, trophyCount } from "./segment-math";
import { useSegmentEfforts } from "./use-segment-efforts";

/** Gold, silver or bronze trophy for the first three places; nothing below. */
export function RankTrophy({ rank, className }: { rank: number; className?: string }) {
  if (rank > TROPHY_RANKS) return null;
  return (
    <Trophy
      className={cn("size-4 shrink-0", className)}
      style={{ color: TROPHY_COLORS[rank - 1] }}
      aria-label={t("Place {{rank}}", { rank })}
    />
  );
}

/**
 * Trophy with the number of segments where the activity is among the fastest three; `labelled`
 * shows it as "2 × Top 3".
 */
export function TrophyBadge({
  activityId,
  enabled = true,
  labelled = false,
  className,
}: {
  activityId: number;
  enabled?: boolean;
  labelled?: boolean;
  className?: string;
}) {
  const { data: efforts = [] } = useSegmentEfforts(activityId, enabled);
  const count = trophyCount(efforts);
  if (count === 0) return null;
  const label = t("Top 3 on {{count}} segments", { count });
  return (
    <span
      title={label}
      aria-label={label}
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-full bg-[#e5b31b]/15 px-2 py-0.5 text-xs font-semibold text-[#b8890a] tabular-nums dark:text-[#f0c53a]",
        className,
      )}
    >
      <Trophy className="size-3.5" />
      {labelled ? t("{{count}} × Top 3", { count }) : count}
    </span>
  );
}
