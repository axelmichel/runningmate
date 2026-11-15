import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Archive, ArchiveRestore, Pencil, Plus, Star, Target, Trash2 } from "lucide-react";
import { useState, type CSSProperties } from "react";
import { toast } from "sonner";

import type { Goal } from "@/bindings/Goal";
import type { GoalProgress } from "@/bindings/GoalProgress";
import type { GoalStatus } from "@/bindings/GoalStatus";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { SPORT_COLORS, VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { GoalFormDialog } from "./goal-form-dialog";
import {
  describeGoal,
  formatGoalValue,
  goalHint,
  invalidateGoals,
  windowLabel,
  windowPercent,
} from "./goal-math";
import { useGoalProgress } from "./use-goals";

const STATUS_LABELS: Record<GoalStatus, string> = {
  upcoming: tk("Upcoming"),
  active: tk("Active"),
  achieved: tk("Reached"),
  missed: tk("Missed"),
  ended: tk("Ended"),
};

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-sm font-semibold tabular-nums">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

/** Earlier windows, oldest left, filled where the target was reached. */
function History({ progress }: { progress: GoalProgress }) {
  const { spec } = progress.goal;
  const windows = [...progress.history].reverse();
  if (windows.length === 0) return null;
  return (
    <div className="flex gap-1" aria-label={t("Earlier periods")}>
      {windows.map((window) => {
        const reached = window.achievedAt !== null;
        return (
          <div
            key={window.start}
            title={`${windowLabel(spec, window)}: ${formatGoalValue(spec.metric, window.value)}`}
            className={cn("h-5 flex-1 rounded-sm", reached ? "bg-(--goal-color)" : "bg-muted")}
            style={
              reached ? undefined : { opacity: 0.4 + 0.6 * (windowPercent(spec, window) / 100) }
            }
          />
        );
      })}
    </div>
  );
}

function GoalCard({
  progress,
  now,
  onEdit,
  onDelete,
  onOpenActivity,
}: {
  progress: GoalProgress;
  now: number;
  onEdit: () => void;
  onDelete: () => void;
  onOpenActivity: (id: number) => void;
}) {
  const queryClient = useQueryClient();
  const { goal, current, status } = progress;
  const { spec } = goal;
  const Icon = VIEW_MODES.find((v) => v.mode === goal.view)?.icon ?? VIEW_MODES[0].icon;
  const style = { "--goal-color": SPORT_COLORS[goal.view] } as CSSProperties;
  const archive = useMutation({
    mutationFn: () => api.archiveGoal(goal.id, !goal.archived),
    onSuccess: () => invalidateGoals(queryClient),
    onError: (error) => toast.error(t("Could not save: {{error}}", { error: String(error) })),
  });
  const hint = goalHint(progress, now);
  const range = goal.end
    ? `${formatDate(Date.parse(`${goal.start}T00:00`) / 1000)} – ${formatDate(Date.parse(`${goal.end}T00:00`) / 1000)}`
    : t("Since {{date}}", { date: formatDate(Date.parse(`${goal.start}T00:00`) / 1000) });

  return (
    <Card
      className={cn("grid grid-cols-[minmax(0,1fr)] gap-4 p-4", goal.archived && "opacity-70")}
      style={style}
    >
      <div className="flex items-start gap-3">
        <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-muted text-(--goal-color)">
          <Icon className="size-5" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <span className="min-w-0 truncate font-semibold">{goal.name}</span>
            <Badge variant={status === "achieved" ? "default" : "secondary"}>
              {t(STATUS_LABELS[status])}
            </Badge>
            {goal.archived && <Badge variant="outline">{t("Archived")}</Badge>}
          </div>
          <div className="line-clamp-2 text-xs text-muted-foreground">
            {describeGoal(spec)} · {range}
          </div>
        </div>
        <div className="flex shrink-0">
          <Button
            variant="ghost"
            size="icon-sm"
            title={t("Edit")}
            aria-label={t("Edit")}
            onClick={onEdit}
          >
            <Pencil />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            title={goal.archived ? t("Restore") : t("Archive")}
            aria-label={goal.archived ? t("Restore") : t("Archive")}
            disabled={archive.isPending}
            onClick={() => archive.mutate()}
          >
            {goal.archived ? <ArchiveRestore /> : <Archive />}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            title={t("Delete")}
            aria-label={t("Delete")}
            onClick={onDelete}
          >
            <Trash2 />
          </Button>
        </div>
      </div>
      <div className="grid gap-1.5">
        <div className="flex items-baseline justify-between gap-2 text-xs text-muted-foreground">
          <span>{windowLabel(spec, current)}</span>
          {hint && <span className="truncate">{hint}</span>}
        </div>
        <div className="text-lg font-semibold tabular-nums">
          {spec.perActivity && current.achievedAt === null && current.value > 0 && (
            <span className="mr-1 text-sm font-normal text-muted-foreground">
              {t("Best so far")}
            </span>
          )}
          {formatGoalValue(spec.metric, current.value)}
          <span className="text-sm font-normal text-muted-foreground">
            {" "}
            / {formatGoalValue(spec.metric, spec.target)}
          </span>
        </div>
        <Progress
          value={windowPercent(spec, current)}
          className="h-2 **:data-[slot=progress-indicator]:bg-(--goal-color)"
        />
      </div>
      <History progress={progress} />
      <div className="flex flex-wrap items-end gap-x-8 gap-y-2">
        {spec.repeat && (
          <>
            <Stat
              label={t("Periods reached")}
              value={`${formatNumber(progress.achieved)} / ${formatNumber(progress.windows)}`}
            />
            <Stat label={t("In a row")} value={formatNumber(progress.streak)} />
          </>
        )}
        <Stat label={t("Activities with a star")} value={formatNumber(progress.hits)} />
        {progress.lastHit && (
          <button
            type="button"
            title={t("Open details")}
            className="ml-auto flex min-w-0 items-center gap-1.5 rounded-md text-left text-xs text-muted-foreground hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
            onClick={() => onOpenActivity(progress.lastHit!.id)}
          >
            <Star className="size-3.5 shrink-0 fill-current text-primary" />
            <span className="truncate">
              {progress.lastHit.title || t("Activity")} · {formatDate(progress.lastHit.date)}
            </span>
          </button>
        )}
      </div>
    </Card>
  );
}

/** Every goal with its progress, by sport; new goals are made here. */
export function GoalsPage({ dataDir }: { dataDir?: string }) {
  const queryClient = useQueryClient();
  const [now] = useState(() => Math.floor(Date.now() / 1000));
  const [view, setView] = useState<ViewMode>("All");
  const [showArchived, setShowArchived] = useState(false);
  const [editing, setEditing] = useState<Goal | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [deleting, setDeleting] = useState<Goal | null>(null);
  const [activityId, setActivityId] = useState<number | null>(null);
  const { data: goals, isPending } = useGoalProgress(showArchived);
  const shown = (goals ?? []).filter((p) => view === "All" || p.goal.view === view);

  const remove = useMutation({
    mutationFn: (id: number) => api.deleteGoal(id),
    onSuccess: () => {
      invalidateGoals(queryClient);
      toast.success(t("Goal deleted"));
    },
    onError: (error) => toast.error(t("Could not delete: {{error}}", { error: String(error) })),
  });

  return (
    <div className="h-full overflow-y-auto">
      <div className="grid gap-6 p-4">
        <div className="flex flex-wrap items-center gap-2">
          {VIEW_MODES.map(({ mode, label, icon: Icon }) => (
            <Button
              key={mode}
              size="sm"
              variant={view === mode ? "default" : "outline"}
              onClick={() => setView(mode)}
            >
              <Icon />
              {t(label)}
            </Button>
          ))}
          <div className="ml-2 flex items-center gap-2">
            <Checkbox
              id="goals-archived"
              checked={showArchived}
              onCheckedChange={(checked) => setShowArchived(checked === true)}
            />
            <Label htmlFor="goals-archived" className="font-normal">
              {t("Show archived")}
            </Label>
          </div>
          <Button
            size="sm"
            className="ml-auto"
            onClick={() => {
              setEditing(null);
              setFormOpen(true);
            }}
          >
            <Plus />
            {t("New goal")}
          </Button>
        </div>
        {isPending ? (
          <Skeleton className="h-64 w-full" />
        ) : shown.length === 0 ? (
          <div className="flex flex-col items-center gap-2 py-16 text-center text-sm text-muted-foreground">
            <Target className="size-8" />
            {t("No goals yet. Set one for a distance, a time or a number of activities.")}
          </div>
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(min(24rem,100%),1fr))] gap-4">
            {shown.map((progress) => (
              <GoalCard
                key={progress.goal.id}
                progress={progress}
                now={now}
                onEdit={() => {
                  setEditing(progress.goal);
                  setFormOpen(true);
                }}
                onDelete={() => setDeleting(progress.goal)}
                onOpenActivity={setActivityId}
              />
            ))}
          </div>
        )}
      </div>
      <GoalFormDialog open={formOpen} goal={editing} onClose={() => setFormOpen(false)} />
      <AlertDialog open={deleting !== null} onOpenChange={(open) => !open && setDeleting(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Delete this goal?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "The goal and the stars it gave are removed; the activities stay as they are. Archive it instead to keep them.",
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => deleting && remove.mutate(deleting.id)}
            >
              {t("Delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      <ActivityDetailDialog
        activityId={activityId}
        onClose={() => setActivityId(null)}
        dataDir={dataDir}
      />
    </div>
  );
}
