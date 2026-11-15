import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TriangleAlert } from "lucide-react";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { Preview } from "@/bindings/Preview";
import type { TrainingPlan } from "@/bindings/TrainingPlan";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { invalidateGoals } from "@/features/goals/goal-math";
import { parseLocalDay } from "@/features/dashboard/dashboard";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import {
  KIND_COLORS,
  KIND_LABELS,
  LENGTHS,
  PHASE_LABELS,
  RACES,
  TARGETS,
  emptyPlanForm,
  invalidateTraining,
  paceRangeLabel,
  planInput,
  suggestedName,
  validatePlan,
  warningText,
  weekdayNames,
  type PlanForm,
} from "./training-math";

function Field({
  id,
  label,
  error,
  hint,
  children,
}: {
  id?: string;
  label: string;
  error?: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="grid content-start gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {error ? (
        <p className="text-xs text-destructive">{t(error)}</p>
      ) : (
        hint && <p className="text-xs text-muted-foreground">{hint}</p>
      )}
    </div>
  );
}

/** Buttons for one choice out of a few. */
function Choices<T extends string | number>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: string; hint?: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      {options.map((option) => (
        <Button
          key={option.value}
          type="button"
          size="sm"
          title={option.hint}
          variant={value === option.value ? "default" : "outline"}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </Button>
      ))}
    </div>
  );
}

const day = (value: string) => (parseLocalDay(value)?.getTime() ?? 0) / 1000;

function TargetStep({ form, set, show }: StepProps) {
  return (
    <div className="grid gap-4">
      <Field label={t("Training for")}>
        <Choices
          value={form.target}
          options={TARGETS.map(({ target, label, hint }) => ({
            value: target,
            label: t(label),
            hint: t(hint),
          }))}
          onChange={(target) => set({ target })}
        />
        <p className="text-xs text-muted-foreground">
          {t(TARGETS.find((x) => x.target === form.target)!.hint)}
        </p>
      </Field>
      {form.target === "race" && (
        <>
          <Field label={t("Distance")} error={show("distance")}>
            <Choices
              value={form.race}
              options={RACES.map(({ race, label }) => ({ value: race, label: t(label) }))}
              onChange={(race) => set({ race })}
            />
            {form.race === "custom" && (
              <Input
                aria-label={t("Distance in km")}
                placeholder="km"
                inputMode="decimal"
                className="w-32"
                value={form.distance}
                onChange={(event) => set({ distance: event.target.value })}
              />
            )}
          </Field>
          <Field
            id="plan-time"
            label={t("Target time (optional)")}
            error={show("targetTime")}
            hint={t("Sets the race pace; without it the pace comes from your runs")}
          >
            <Input
              id="plan-time"
              className="w-40"
              placeholder="H:MM:SS"
              value={form.targetTime}
              onChange={(event) => set({ targetTime: event.target.value })}
            />
          </Field>
        </>
      )}
    </div>
  );
}

function ScheduleStep({ form, set, show }: StepProps) {
  const names = weekdayNames("short");
  const toggle = (d: number) =>
    set({ days: form.days.includes(d) ? form.days.filter((x) => x !== d) : [...form.days, d] });
  return (
    <div className="grid gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id="plan-start" label={t("Start")} error={show("start")}>
          <Input
            id="plan-start"
            type="date"
            value={form.start}
            onChange={(event) => set({ start: event.target.value })}
          />
        </Field>
        <Field id="plan-weeks" label={t("Weeks")} error={show("weeks")}>
          <div className="flex gap-2">
            <Input
              id="plan-weeks"
              className="w-20"
              inputMode="numeric"
              value={form.weeks}
              onChange={(event) => set({ weeks: event.target.value })}
            />
            <Choices
              value={Number(form.weeks)}
              options={LENGTHS.map((weeks) => ({
                value: weeks,
                label: weeks === 12 ? t("3 months") : String(weeks),
              }))}
              onChange={(weeks) => set({ weeks: String(weeks) })}
            />
          </div>
        </Field>
      </div>
      <Field label={t("Training days")} error={show("days")}>
        <div className="flex gap-1">
          {names.map((name, d) => (
            <Button
              key={d}
              type="button"
              size="sm"
              className="w-12"
              variant={form.days.includes(d) ? "default" : "outline"}
              onClick={() => toggle(d)}
            >
              {name}
            </Button>
          ))}
        </div>
      </Field>
      <Field
        label={t("Long run")}
        error={show("longDay")}
        hint={form.target === "race" ? t("The race is on this day of the last week") : undefined}
      >
        <Choices
          value={form.longDay}
          options={[...form.days].sort((a, b) => a - b).map((d) => ({ value: d, label: names[d] }))}
          onChange={(longDay) => set({ longDay })}
        />
      </Field>
      <Field id="plan-name" label={t("Name")}>
        <Input
          id="plan-name"
          value={form.name}
          placeholder={suggestedName(form)}
          onChange={(event) => set({ name: event.target.value })}
        />
      </Field>
    </div>
  );
}

/** The weeks of the preview, one row each with its sessions. */
export function PreviewWeeks({ preview }: { preview: Preview }) {
  const weeks = new Map<number, Preview["sessions"]>();
  for (const session of preview.sessions) {
    weeks.set(session.week, [...(weeks.get(session.week) ?? []), session]);
  }
  return (
    <div className="grid max-h-64 gap-1 overflow-y-auto pr-1 text-xs">
      {[...weeks].map(([week, sessions]) => (
        <div key={week} className="flex items-center gap-2">
          <span className="w-16 shrink-0 text-muted-foreground">
            {t("Week {{week}}", { week: week + 1 })}
          </span>
          <span className="w-20 shrink-0 truncate">{t(PHASE_LABELS[sessions[0].phase])}</span>
          <span className="w-14 shrink-0 text-right tabular-nums">
            {formatNumber(
              sessions.reduce((sum, s) => sum + s.distance, 0),
              0,
            )}{" "}
            km
          </span>
          <div className="flex min-w-0 flex-1 gap-1">
            {sessions.map((s) => (
              <span
                key={s.date}
                title={`${formatDate(day(s.date))}: ${t(KIND_LABELS[s.kind])}, ${formatNumber(s.distance, 1)} km`}
                className="h-4 rounded-sm"
                style={{
                  background: KIND_COLORS[s.kind],
                  width: `${Math.max(8, s.distance * 2)}px`,
                }}
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function PreviewStep({ form, set }: StepProps) {
  const input = planInput(form);
  const { data: baseline } = useQuery({
    queryKey: ["training", "baseline"],
    queryFn: api.trainingBaseline,
  });
  const { data: preview, error } = useQuery({
    queryKey: ["training", "preview", input],
    queryFn: () => api.previewTrainingPlan(input),
  });
  return (
    <div className="grid gap-4">
      {baseline && (
        <div className="rounded-md bg-muted/50 p-3 text-xs">
          <div className="mb-1 font-medium">{t("Your last 6 months")}</div>
          {baseline.runs === 0
            ? t("No runs yet; the plan starts gently.")
            : [
                t("{{km}} km a week lately", { km: formatNumber(baseline.weeklyKm, 0) }),
                t("{{runs}} runs a week", { runs: formatNumber(baseline.runsPerWeek, 1) }),
                t("longest run {{km}} km", { km: formatNumber(baseline.longestKm, 1) }),
                baseline.vdot !== null && `VO2max ${formatNumber(baseline.vdot, 0)}`,
              ]
                .filter(Boolean)
                .join(" · ")}
        </div>
      )}
      {error ? (
        <p className="text-xs text-destructive">{String(error)}</p>
      ) : !preview ? (
        <Skeleton className="h-40 w-full" />
      ) : (
        <>
          {preview.warnings.map((warning) => (
            <div
              key={warning.code}
              className="flex gap-2 text-xs text-amber-700 dark:text-amber-400"
            >
              <TriangleAlert className="size-4 shrink-0" />
              {warningText(warning)}
            </div>
          ))}
          <div className="text-xs text-muted-foreground">
            {t("{{sessions}} sessions until {{end}}", {
              sessions: preview.sessions.length,
              end: formatDate(day(preview.end)),
            })}
            {preview.paces &&
              ` · ${t("easy")} ${paceRangeLabel(preview.paces.easy)} · ${t("threshold")} ${paceRangeLabel(preview.paces.threshold)}`}
          </div>
          <PreviewWeeks preview={preview} />
        </>
      )}
      <div className="flex items-center gap-2">
        <Checkbox
          id="plan-goal"
          checked={form.createGoal}
          onCheckedChange={(checked) => set({ createGoal: checked === true })}
        />
        <Label htmlFor="plan-goal" className="font-normal">
          {form.target === "race"
            ? t("Also set a goal: the race distance by the race day")
            : t("Also set a goal: the plan's weekly km every week")}
        </Label>
      </div>
    </div>
  );
}

type StepProps = {
  form: PlanForm;
  set: (change: Partial<PlanForm>) => void;
  show: (key: keyof PlanForm) => string | undefined;
};

const STEPS = [TargetStep, ScheduleStep, PreviewStep];
/** Fields checked before leaving each step. */
const STEP_FIELDS: (keyof PlanForm)[][] = [
  ["distance", "targetTime"],
  ["start", "weeks", "days", "longDay"],
  [],
];

function Wizard({ onDone }: { onDone: (plan?: TrainingPlan) => void }) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState(() => emptyPlanForm());
  const [step, setStep] = useState(0);
  const [touched, setTouched] = useState(false);
  const errors = validatePlan(form);
  const set = (change: Partial<PlanForm>) => setForm((current) => ({ ...current, ...change }));
  const show = (key: keyof PlanForm) => (touched ? errors[key] : undefined);
  const stepValid = STEP_FIELDS[step].every((key) => !errors[key]);
  const Step = STEPS[step];

  const create = useMutation({
    mutationFn: () => api.createTrainingPlan(planInput(form), form.createGoal),
    onSuccess: (plan) => {
      invalidateTraining(queryClient);
      if (form.createGoal) invalidateGoals(queryClient);
      toast.success(t("Training plan created"));
      onDone(plan);
    },
    onError: (error) => toast.error(t("Could not save: {{error}}", { error: String(error) })),
  });

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        setTouched(true);
        if (!stepValid) return;
        setTouched(false);
        if (step < STEPS.length - 1) setStep(step + 1);
        else create.mutate();
      }}
    >
      <div className="flex gap-1">
        {STEPS.map((_, i) => (
          <div
            key={i}
            className={cn("h-1 flex-1 rounded-full bg-muted", i <= step && "bg-primary")}
          />
        ))}
      </div>
      <Step form={form} set={set} show={show} />
      <DialogFooter>
        <Button
          type="button"
          variant="outline"
          onClick={() => (step === 0 ? onDone() : setStep(step - 1))}
        >
          {step === 0 ? t("Cancel") : t("Back")}
        </Button>
        <Button type="submit" disabled={create.isPending}>
          {step < STEPS.length - 1 ? t("Next") : t("Create plan")}
        </Button>
      </DialogFooter>
    </form>
  );
}

/** Makes a new plan in three steps: target, schedule, and a look at the plan. */
export function PlanWizardDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: (plan?: TrainingPlan) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("New training plan")}</DialogTitle>
          <DialogDescription>
            {t(
              "The plan builds on your runs of the last 6 months: an easy start, a long run, and faster sessions that get you closer to the target.",
            )}
          </DialogDescription>
        </DialogHeader>
        {open && <Wizard onDone={onClose} />}
      </DialogContent>
    </Dialog>
  );
}
