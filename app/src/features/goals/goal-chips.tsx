import { Plus, Target } from "lucide-react";
import { useState } from "react";

import { t } from "@/i18n";

import { GoalChip } from "./goal-chip";
import { useGoalProgress } from "./use-goals";

/**
 * The dashboard row: a chip per goal that is running or reached, opening the goals page. With
 * `at`, the goals as they stood then.
 */
export function GoalChips({ onGoals, at }: { onGoals: () => void; at?: number }) {
  const [mounted] = useState(() => Math.floor(Date.now() / 1000));
  const now = at ?? mounted;
  const { data: goals, isPending } = useGoalProgress(false, at);
  if (isPending) return null;
  const shown = (goals ?? []).filter((p) => p.status !== "missed" && p.status !== "ended");

  return (
    // Wraps instead of scrolling sideways, which put a scrollbar over the bottom of the chips;
    // every row as tall as the tallest chip, so all chips have the same height.
    <div className="grid auto-rows-fr grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-2">
      {shown.map((progress) => (
        <GoalChip key={progress.goal.id} progress={progress} now={now} onClick={onGoals} />
      ))}
      <button
        type="button"
        className="flex flex-col items-center justify-center gap-1 rounded-lg border border-dashed px-4 py-2 text-xs text-muted-foreground transition-colors hover:bg-muted/40 hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={onGoals}
      >
        {shown.length === 0 ? <Target className="size-4" /> : <Plus className="size-4" />}
        {shown.length === 0 ? t("Set a goal") : t("Goals")}
      </button>
    </div>
  );
}
