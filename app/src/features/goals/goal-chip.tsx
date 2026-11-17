import { Star } from "lucide-react";
import type { CSSProperties } from "react";

import type { GoalProgress } from "@/bindings/GoalProgress";
import { Progress } from "@/components/ui/progress";
import { SPORT_COLORS, VIEW_MODES } from "@/features/activities/view-mode";
import { t } from "@/i18n";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

import { formatGoalValue, goalHint, windowPercent } from "./goal-math";

/** Small card with a goal's name, the current window's progress and its state. */
export function GoalChip({
  progress,
  now,
  onClick,
  className,
}: {
  progress: GoalProgress;
  now: number;
  onClick?: () => void;
  className?: string;
}) {
  const { goal, current, status } = progress;
  const { spec } = goal;
  const Icon = VIEW_MODES.find((v) => v.mode === goal.view)?.icon ?? VIEW_MODES[0].icon;
  const reached = current.achievedAt !== null;
  const value = formatGoalValue(spec.metric, current.value);
  const target = formatGoalValue(spec.metric, spec.target);
  const hint = goalHint(progress, now);
  const style = { "--goal-color": SPORT_COLORS[goal.view] } as CSSProperties;
  // A distance one run has to cover counts its hits instead of showing one run's distance.
  const counted = spec.metric === "distance" && spec.perActivity;

  return (
    <button
      type="button"
      title={goal.name}
      style={style}
      disabled={!onClick}
      className={cn(
        "flex min-w-0 flex-col gap-1.5 rounded-lg border bg-card px-3 py-2 text-left transition-colors enabled:hover:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none",
        (status === "missed" || status === "ended") && "opacity-60",
        className,
      )}
      onClick={onClick}
    >
      <div className="flex min-w-0 items-center gap-1.5 text-xs font-semibold">
        <Icon className="size-3.5 shrink-0 text-(--goal-color)" />
        <span className="truncate">{goal.name}</span>
        {reached && (
          <Star
            className="ml-auto size-3.5 shrink-0 fill-current text-primary"
            aria-label={t("Reached")}
          />
        )}
      </div>
      {counted ? (
        <>
          <div className="flex items-baseline gap-1.5">
            <span className="text-2xl leading-none font-semibold tabular-nums">
              {progress.hits}
            </span>
            <span className="text-xs text-muted-foreground">
              {t("times reached")} · {target}
            </span>
          </div>
          <div className="truncate text-xs text-muted-foreground tabular-nums">
            {progress.bestDuration !== null
              ? t("Best time {{time}}", { time: formatDuration(progress.bestDuration) })
              : t("Not reached yet")}
          </div>
        </>
      ) : (
        <>
          <div className="text-sm font-semibold tabular-nums">
            {spec.perActivity && !reached && current.value > 0 && (
              <span className="mr-1 text-xs font-normal text-muted-foreground">
                {t("Best so far")}
              </span>
            )}
            {value}
            <span className="font-normal text-muted-foreground"> / {target}</span>
          </div>
          <Progress
            value={windowPercent(spec, current)}
            className="h-1.5 **:data-[slot=progress-indicator]:bg-(--goal-color)"
          />
        </>
      )}
      {/* At the bottom, so it lines up across chips of different content. */}
      <div className="mt-auto truncate text-xs text-muted-foreground">
        {[
          (!counted || !reached) && hint,
          progress.streak > 1 && t("{{count}} in a row", { count: progress.streak }),
        ]
          .filter(Boolean)
          .join(" · ") || " "}
      </div>
    </button>
  );
}
