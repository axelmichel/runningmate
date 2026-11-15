import { Star } from "lucide-react";

import { t } from "@/i18n";
import { cn } from "@/lib/utils";

import { useGoalHits } from "./use-goals";

/** Star for an activity that hit a goal, with the count when it hit several. */
export function GoalStar({
  activityId,
  enabled = true,
  className,
}: {
  activityId: number;
  enabled?: boolean;
  className?: string;
}) {
  const { data: hits = [] } = useGoalHits(activityId, enabled);
  if (hits.length === 0) return null;
  const label = t("Goal reached: {{goals}}", { goals: hits.map((hit) => hit.name).join(", ") });
  return (
    <span
      title={label}
      aria-label={label}
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-full bg-primary/10 px-2 py-0.5 text-xs font-semibold text-primary tabular-nums",
        className,
      )}
    >
      <Star className="size-3.5 fill-current" />
      {hits.length > 1 && hits.length}
    </span>
  );
}
