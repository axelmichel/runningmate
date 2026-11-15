import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Archive,
  ArchiveRestore,
  CalendarCheck,
  ExternalLink,
  Pencil,
  Plus,
  RefreshCw,
  Target,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { PlanDetail } from "@/bindings/PlanDetail";
import type { PlanStatus } from "@/bindings/PlanStatus";
import type { PlanSummary } from "@/bindings/PlanSummary";
import type { SessionStatus } from "@/bindings/SessionStatus";
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
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { localDay, parseLocalDay } from "@/features/dashboard/dashboard";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { CoachNote } from "./coach-note";
import { PlanWizardDialog } from "./plan-wizard-dialog";
import { SessionDialog } from "./session-dialog";
import {
  KIND_COLORS,
  KIND_LABELS,
  PHASE_LABELS,
  VERDICT_COLORS,
  addDays,
  checkText,
  invalidateTraining,
  paceRangeLabel,
  sessionDetail,
  targetLabel,
  weekdayNames,
} from "./training-math";
import { useTrainingPlan, useTrainingPlans } from "./use-training";

const STATUS_LABELS: Record<PlanStatus, string> = {
  upcoming: tk("Upcoming"),
  active: tk("Active"),
  finished: tk("Finished"),
};

const PHASE_COLORS = {
  base: "var(--chart-5)",
  build: "var(--chart-4)",
  peak: "var(--chart-3)",
  taper: "var(--chart-2)",
  recovery: "var(--muted)",
} as const;

const unixDay = (day: string) => (parseLocalDay(day)?.getTime() ?? 0) / 1000;

function onError(error: unknown) {
  toast.error(t("Could not save: {{error}}", { error: String(error) }));
}

function PlanButton({
  summary,
  selected,
  onClick,
}: {
  summary: PlanSummary;
  selected: boolean;
  onClick: () => void;
}) {
  const { plan } = summary;
  return (
    <button
      type="button"
      className={cn(
        "grid w-56 shrink-0 gap-1 rounded-lg border p-3 text-left transition-colors hover:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none",
        selected && "border-primary bg-primary/5",
        plan.archived && "opacity-70",
      )}
      onClick={onClick}
    >
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate text-sm font-semibold">{plan.name}</span>
        <Badge variant={summary.status === "active" ? "default" : "secondary"}>
          {t(STATUS_LABELS[summary.status])}
        </Badge>
      </div>
      <div className="truncate text-xs text-muted-foreground">
        {formatDate(unixDay(plan.start))} – {formatDate(unixDay(plan.end))}
      </div>
      <Progress
        value={summary.sessions ? (summary.trained / summary.sessions) * 100 : 0}
        className="h-1.5"
      />
    </button>
  );
}

function SessionCard({
  status,
  today,
  onEdit,
  onOpenRun,
}: {
  status: SessionStatus;
  today: string;
  onEdit: () => void;
  onOpenRun: (id: number) => void;
}) {
  const { session, check, run } = status;
  const past = session.date < today;
  return (
    <div
      className={cn(
        "group relative grid gap-0.5 rounded-md border-l-4 bg-muted/40 px-2 py-1.5 text-left text-xs",
        session.skipped && "opacity-50",
      )}
      style={{ borderLeftColor: KIND_COLORS[session.kind] }}
    >
      <button
        type="button"
        className="absolute inset-0 rounded-md focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        aria-label={t("Edit")}
        onClick={onEdit}
      />
      <div className="flex items-center gap-1 font-medium">
        <span className={cn("min-w-0 flex-1 truncate", session.skipped && "line-through")}>
          {t(KIND_LABELS[session.kind])}
        </span>
        {run && (
          <button
            type="button"
            className="relative text-muted-foreground hover:text-foreground"
            title={t("Open details")}
            aria-label={t("Open details")}
            onClick={() => onOpenRun(run.id)}
          >
            <ExternalLink className="size-3" />
          </button>
        )}
      </div>
      <div className="truncate text-muted-foreground" title={sessionDetail(session)}>
        {sessionDetail(session)}
      </div>
      {(run || past || session.skipped) && (
        <div className={cn("truncate", VERDICT_COLORS[check.verdict])} title={checkText(status)}>
          {checkText(status)}
        </div>
      )}
      {session.note && <div className="truncate italic">{session.note}</div>}
    </div>
  );
}

/** The plan's weeks as rows of seven days, starting on the plan's first weekday. */
function PlanCalendar({
  detail,
  today,
  onEdit,
  onAdd,
  onOpenRun,
}: {
  detail: PlanDetail;
  today: string;
  onEdit: (status: SessionStatus) => void;
  onAdd: (date: string) => void;
  onOpenRun: (id: number) => void;
}) {
  const { plan } = detail.summary;
  const byDate = new Map<string, SessionStatus[]>();
  for (const status of detail.sessions) {
    byDate.set(status.session.date, [...(byDate.get(status.session.date) ?? []), status]);
  }
  const names = weekdayNames("short");
  const firstWeekday = (parseLocalDay(plan.start)!.getDay() + 6) % 7;

  return (
    <div className="grid gap-2">
      <div className="grid grid-cols-[7rem_repeat(7,minmax(0,1fr))] gap-1 text-xs text-muted-foreground">
        <div />
        {Array.from({ length: 7 }, (_, i) => (
          <div key={i} className="px-1">
            {names[(firstWeekday + i) % 7]}
          </div>
        ))}
      </div>
      {detail.weeks.map((week) => {
        const current = today >= week.start && today < addDays(week.start, 7);
        return (
          <div
            key={week.week}
            className={cn(
              "grid grid-cols-[7rem_repeat(7,minmax(0,1fr))] gap-1 rounded-lg p-1",
              current && "bg-primary/5 ring-1 ring-primary/30",
            )}
          >
            <div className="grid content-start gap-0.5 px-1 text-xs">
              <span className="font-medium">{t("Week {{week}}", { week: week.week + 1 })}</span>
              <span className="text-muted-foreground">{t(PHASE_LABELS[week.phase])}</span>
              <span className="tabular-nums text-muted-foreground">
                {formatNumber(week.doneKm, 0)} / {formatNumber(week.plannedKm, 0)} km
              </span>
            </div>
            {Array.from({ length: 7 }, (_, i) => {
              const date = addDays(week.start, i);
              const sessions = byDate.get(date) ?? [];
              const inPlan = date <= plan.end;
              return (
                <div
                  key={date}
                  className={cn(
                    "group/day grid min-h-20 content-start gap-1 rounded-md border border-transparent p-1",
                    date === today && "border-primary/50",
                    !inPlan && "opacity-30",
                  )}
                >
                  <div className="flex items-center justify-between text-[0.7rem] text-muted-foreground">
                    <span>{parseLocalDay(date)!.getDate()}</span>
                    {inPlan && (
                      <button
                        type="button"
                        className="opacity-0 transition-opacity group-hover/day:opacity-100 focus-visible:opacity-100"
                        title={t("Add session")}
                        aria-label={t("Add session")}
                        onClick={() => onAdd(date)}
                      >
                        <Plus className="size-3" />
                      </button>
                    )}
                  </div>
                  {sessions.map((status) => (
                    <SessionCard
                      key={status.session.id}
                      status={status}
                      today={today}
                      onEdit={() => onEdit(status)}
                      onOpenRun={onOpenRun}
                    />
                  ))}
                </div>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}

function RenameDialog({
  detail,
  open,
  onClose,
}: {
  detail: PlanDetail;
  open: boolean;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [name, setName] = useState(detail.summary.plan.name);
  const rename = useMutation({
    mutationFn: () => api.renameTrainingPlan(detail.summary.plan.id, name),
    onSuccess: () => {
      invalidateTraining(queryClient);
      onClose();
    },
    onError,
  });
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{t("Rename plan")}</DialogTitle>
        </DialogHeader>
        <form
          className="grid gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            rename.mutate();
          }}
        >
          <Input aria-label={t("Name")} value={name} onChange={(e) => setName(e.target.value)} />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              {t("Cancel")}
            </Button>
            <Button type="submit" disabled={!name.trim() || rename.isPending}>
              {t("Save")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function PlanView({
  detail,
  onDeleted,
  onOpenRun,
}: {
  detail: PlanDetail;
  onDeleted: () => void;
  onOpenRun: (id: number) => void;
}) {
  const queryClient = useQueryClient();
  const { summary, paces, goal } = detail;
  const { plan } = summary;
  const [today] = useState(() => localDay(new Date()));
  const [editing, setEditing] = useState<SessionStatus | null>(null);
  const [adding, setAdding] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const adapt = useMutation({
    mutationFn: () => api.adaptTrainingPlan(plan.id),
    onSuccess: () => {
      invalidateTraining(queryClient);
      toast.success(t("Plan adapted to your recent runs"));
    },
    onError,
  });
  const archive = useMutation({
    mutationFn: () => api.archiveTrainingPlan(plan.id, !plan.archived),
    onSuccess: () => invalidateTraining(queryClient),
    onError,
  });
  const remove = useMutation({
    mutationFn: () => api.deleteTrainingPlan(plan.id),
    onSuccess: () => {
      invalidateTraining(queryClient);
      toast.success(t("Training plan deleted"));
      onDeleted();
    },
    onError: (error) => toast.error(t("Could not delete: {{error}}", { error: String(error) })),
  });

  // The week holding today, or the first one before the plan starts and the last after it.
  const currentWeek =
    detail.weeks.filter((week) => week.start <= today).at(-1)?.week ?? detail.weeks[0]?.week ?? 0;

  const paceRows = paces
    ? ([
        [t("Easy"), paces.easy],
        [t("Marathon pace"), paces.marathon],
        [t("Threshold"), paces.threshold],
        [t("Interval"), paces.interval],
      ] as const)
    : [];

  return (
    <div className="grid gap-4">
      <Card className="grid gap-4 p-4">
        <div className="flex items-start gap-3">
          <div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-muted text-primary">
            <CalendarCheck className="size-5" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="truncate font-semibold">{plan.name}</span>
              <Badge variant={summary.status === "active" ? "default" : "secondary"}>
                {t(STATUS_LABELS[summary.status])}
              </Badge>
              {plan.archived && <Badge variant="outline">{t("Archived")}</Badge>}
              {goal && (
                <Badge variant="outline" title={t("Matching goal")}>
                  <Target />
                  {goal.name}
                </Badge>
              )}
            </div>
            <div className="truncate text-xs text-muted-foreground">
              {targetLabel(plan)} · {formatDate(unixDay(plan.start))} –{" "}
              {formatDate(unixDay(plan.end))} · {t("{{weeks}} weeks", { weeks: plan.weeks })}
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <Button
              size="sm"
              variant="outline"
              disabled={adapt.isPending || summary.status === "finished"}
              title={t(
                "Rebuild the sessions from today on from your recent runs; changed sessions stay",
              )}
              onClick={() => adapt.mutate()}
            >
              <RefreshCw className={cn(adapt.isPending && "animate-spin")} />
              {t("Adapt plan")}
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title={t("Rename")}
              aria-label={t("Rename")}
              onClick={() => setRenaming(true)}
            >
              <Pencil />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title={plan.archived ? t("Restore") : t("Archive")}
              aria-label={plan.archived ? t("Restore") : t("Archive")}
              disabled={archive.isPending}
              onClick={() => archive.mutate()}
            >
              {plan.archived ? <ArchiveRestore /> : <Archive />}
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              title={t("Delete")}
              aria-label={t("Delete")}
              onClick={() => setDeleting(true)}
            >
              <Trash2 />
            </Button>
          </div>
        </div>
        <div className="flex flex-wrap items-end gap-x-8 gap-y-2">
          <Stat
            label={t("Sessions with a run")}
            value={`${formatNumber(summary.trained)} / ${formatNumber(summary.sessions)}`}
          />
          <Stat label={t("Done as planned")} value={formatNumber(summary.done)} />
          <Stat label={t("Missed")} value={formatNumber(summary.missed)} />
          <Stat
            label={t("Distance")}
            value={`${formatNumber(summary.doneKm, 0)} / ${formatNumber(summary.plannedKm, 0)} km`}
          />
          {paceRows.map(([label, range]) => (
            <Stat key={label} label={label} value={paceRangeLabel(range)} />
          ))}
        </div>
        <div className="flex gap-0.5" aria-label={t("Phases")}>
          {detail.weeks.map((week) => (
            <div
              key={week.week}
              title={`${t("Week {{week}}", { week: week.week + 1 })}: ${t(PHASE_LABELS[week.phase])}`}
              className="h-2 flex-1 rounded-sm"
              style={{ background: PHASE_COLORS[week.phase] }}
            />
          ))}
        </div>
      </Card>
      <CoachNote planId={plan.id} week={currentWeek} />
      <PlanCalendar
        detail={detail}
        today={today}
        onEdit={setEditing}
        onAdd={setAdding}
        onOpenRun={onOpenRun}
      />
      <SessionDialog
        open={editing !== null || adding !== null}
        status={editing}
        planId={plan.id}
        date={adding ?? today}
        onClose={() => {
          setEditing(null);
          setAdding(null);
        }}
      />
      {renaming && <RenameDialog detail={detail} open onClose={() => setRenaming(false)} />}
      <AlertDialog open={deleting} onOpenChange={setDeleting}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Delete this training plan?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "The plan and its sessions are removed; your runs and a matching goal stay. Archive it instead to keep it.",
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => remove.mutate()}>
              {t("Delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-sm font-semibold tabular-nums">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

/** The plan to show first: the running one, else the next, else the latest. */
function firstPlan(plans: PlanSummary[]): number | null {
  const by = (status: PlanStatus) => plans.find((p) => p.status === status && !p.plan.archived);
  return (by("active") ?? by("upcoming") ?? plans[0])?.plan.id ?? null;
}

/** Training plans with their sessions; new plans are made here. */
export function TrainingPage({ dataDir }: { dataDir?: string }) {
  const [showArchived, setShowArchived] = useState(false);
  const [picked, setPicked] = useState<number | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [activityId, setActivityId] = useState<number | null>(null);
  const { data: plans = [], isPending } = useTrainingPlans(showArchived);
  const selected = plans.some((p) => p.plan.id === picked) ? picked : firstPlan(plans);
  const { data: detail } = useTrainingPlan(selected);

  return (
    <div className="h-full overflow-y-auto">
      <div className="grid gap-6 p-4">
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-2">
            <Checkbox
              id="training-archived"
              checked={showArchived}
              onCheckedChange={(checked) => setShowArchived(checked === true)}
            />
            <Label htmlFor="training-archived" className="font-normal">
              {t("Show archived")}
            </Label>
          </div>
          <Button size="sm" className="ml-auto" onClick={() => setWizardOpen(true)}>
            <Plus />
            {t("New training plan")}
          </Button>
        </div>
        {isPending ? (
          <Skeleton className="h-64 w-full" />
        ) : plans.length === 0 ? (
          <div className="flex flex-col items-center gap-2 py-16 text-center text-sm text-muted-foreground">
            <CalendarCheck className="size-8" />
            {t(
              "No training plan yet. Pick a race or a fitness target, and the app plans the sessions from your recent runs.",
            )}
          </div>
        ) : (
          <>
            {plans.length > 1 && (
              <div className="flex gap-2 overflow-x-auto pb-1">
                {plans.map((summary) => (
                  <PlanButton
                    key={summary.plan.id}
                    summary={summary}
                    selected={summary.plan.id === selected}
                    onClick={() => setPicked(summary.plan.id)}
                  />
                ))}
              </div>
            )}
            {detail ? (
              <PlanView
                key={detail.summary.plan.id}
                detail={detail}
                onDeleted={() => setPicked(null)}
                onOpenRun={setActivityId}
              />
            ) : (
              <Skeleton className="h-64 w-full" />
            )}
          </>
        )}
      </div>
      <PlanWizardDialog
        open={wizardOpen}
        onClose={(plan) => {
          setWizardOpen(false);
          if (plan) setPicked(plan.id);
        }}
      />
      <ActivityDetailDialog
        activityId={activityId}
        onClose={() => setActivityId(null)}
        dataDir={dataDir}
      />
    </div>
  );
}
