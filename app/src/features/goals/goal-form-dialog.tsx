import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { Goal } from "@/bindings/Goal";
import type { GoalMetric } from "@/bindings/GoalMetric";
import type { GoalPeriod } from "@/bindings/GoalPeriod";
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";

import {
  METRICS,
  PERIODS,
  allowsPerActivity,
  allowsTimeLimit,
  describeGoal,
  emptyGoalForm,
  goalForm,
  goalInput,
  goalSpec,
  invalidateGoals,
  validateGoal,
  type GoalForm,
} from "./goal-math";

function Field({
  id,
  label,
  error,
  hint,
  children,
}: {
  id: string;
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

function Choice<T extends string>({
  id,
  value,
  options,
  onChange,
}: {
  id: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <Select value={value} onValueChange={(next) => onChange(next as T)}>
      <SelectTrigger id={id} className="w-full">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {t(option.label)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

function GoalFormView({ goal, onDone }: { goal: Goal | null; onDone: () => void }) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<GoalForm>(() => (goal ? goalForm(goal) : emptyGoalForm()));
  const [touched, setTouched] = useState(false);
  const errors = validateGoal(form);
  const valid = Object.keys(errors).length === 0;
  const set = (change: Partial<GoalForm>) => setForm((current) => ({ ...current, ...change }));
  const show = (key: keyof GoalForm) => (touched ? errors[key] : undefined);
  const unit = METRICS.find((m) => m.metric === form.metric)?.unit;
  const suggestion = valid ? describeGoal(goalSpec(form)) : "";

  const save = useMutation({
    mutationFn: () =>
      goal ? api.updateGoal(goal.id, goalInput(form)) : api.createGoal(goalInput(form)),
    onSuccess: () => {
      invalidateGoals(queryClient);
      toast.success(t("Goal saved"));
      onDone();
    },
    onError: (error) => toast.error(t("Could not save: {{error}}", { error: String(error) })),
  });

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        setTouched(true);
        if (valid) save.mutate();
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id="goal-view" label={t("Sport")}>
          <Choice<ViewMode>
            id="goal-view"
            value={form.view}
            options={VIEW_MODES.map(({ mode, label }) => ({ value: mode, label }))}
            onChange={(view) => set({ view })}
          />
        </Field>
        <Field id="goal-metric" label={t("Goal for")}>
          <Choice<GoalMetric>
            id="goal-metric"
            value={form.metric}
            options={METRICS.map(({ metric, label }) => ({ value: metric, label }))}
            onChange={(metric) => set({ metric })}
          />
        </Field>
        <Field
          id="goal-target"
          label={unit ? `${t("Target")} (${unit})` : t("Target")}
          error={show("target")}
        >
          <Input
            id="goal-target"
            type="number"
            min={0}
            step="any"
            value={form.target}
            aria-invalid={Boolean(show("target"))}
            onChange={(event) => set({ target: event.target.value })}
          />
        </Field>
        <Field id="goal-scope" label={t("Counted")}>
          <Choice<"total" | "activity">
            id="goal-scope"
            value={form.perActivity && allowsPerActivity(form) ? "activity" : "total"}
            options={[
              { value: "total", label: tk("All activities together") },
              ...(allowsPerActivity(form)
                ? [{ value: "activity" as const, label: tk("In one activity") }]
                : []),
            ]}
            onChange={(scope) => set({ perActivity: scope === "activity" })}
          />
        </Field>
        {allowsTimeLimit(form) && (
          <Field
            id="goal-limit"
            label={t("Time limit")}
            error={show("maxDuration")}
            hint={t("Optional, e.g. 0:50:00 for 10 km in under 50 minutes")}
          >
            <Input
              id="goal-limit"
              placeholder="H:MM:SS"
              value={form.maxDuration}
              aria-invalid={Boolean(show("maxDuration"))}
              onChange={(event) => set({ maxDuration: event.target.value })}
            />
          </Field>
        )}
        <Field id="goal-period" label={t("Period")}>
          <Choice<GoalPeriod>
            id="goal-period"
            value={form.period}
            options={PERIODS.map(({ period, label }) => ({ value: period, label }))}
            onChange={(period) => set({ period })}
          />
        </Field>
        {form.period === "days" && (
          <Field id="goal-days" label={t("Days")} error={show("days")}>
            <Input
              id="goal-days"
              type="number"
              min={1}
              step={1}
              value={form.days}
              aria-invalid={Boolean(show("days"))}
              onChange={(event) => set({ days: event.target.value })}
            />
          </Field>
        )}
        <div className="flex items-center gap-2 self-end pb-2 sm:col-span-2">
          <Checkbox
            id="goal-repeat"
            checked={form.repeat && form.period !== "open"}
            disabled={form.period === "open"}
            onCheckedChange={(checked) => set({ repeat: checked === true })}
          />
          <Label htmlFor="goal-repeat" className="font-normal">
            {form.period === "days"
              ? t("Repeat: start over after each window of days")
              : t("Repeat: start over every period")}
          </Label>
        </div>
        <Field
          id="goal-start"
          label={t("Start")}
          error={show("start")}
          hint={
            form.period === "week" || form.period === "month" || form.period === "year"
              ? t("The period holding this day counts in full")
              : undefined
          }
        >
          <Input
            id="goal-start"
            type="date"
            value={form.start}
            aria-invalid={Boolean(show("start"))}
            onChange={(event) => set({ start: event.target.value })}
          />
        </Field>
        <Field id="goal-end" label={t("End")} error={form.endless ? undefined : show("end")}>
          {/* A date field can't be emptied once set, so "no end" is a choice of its own. */}
          <div className="flex items-center gap-3">
            <div className="flex shrink-0 items-center gap-2">
              <Checkbox
                id="goal-endless"
                checked={form.endless}
                onCheckedChange={(checked) =>
                  set({ endless: checked === true, end: form.end || form.start })
                }
              />
              <Label htmlFor="goal-endless" className="font-normal">
                {t("No end date")}
              </Label>
            </div>
            {!form.endless && (
              <Input
                id="goal-end"
                type="date"
                value={form.end}
                aria-invalid={Boolean(show("end"))}
                onChange={(event) => set({ end: event.target.value })}
              />
            )}
          </div>
        </Field>
        <div className="sm:col-span-2">
          <Field id="goal-name" label={t("Name")}>
            <Input
              id="goal-name"
              value={form.name}
              placeholder={suggestion}
              onChange={(event) => set({ name: event.target.value })}
            />
          </Field>
        </div>
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onDone}>
          {t("Cancel")}
        </Button>
        <Button type="submit" disabled={save.isPending}>
          {t("Save")}
        </Button>
      </DialogFooter>
    </form>
  );
}

/** Creates a goal, or edits `goal` when given. */
export function GoalFormDialog({
  open,
  goal,
  onClose,
}: {
  open: boolean;
  goal: Goal | null;
  onClose: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{goal ? t("Edit goal") : t("New goal")}</DialogTitle>
          <DialogDescription>
            {t(
              "A distance, time, climb or number of activities, in one activity or in total, once or repeating.",
            )}
          </DialogDescription>
        </DialogHeader>
        {/* Keyed so the form starts fresh for every goal. */}
        {open && <GoalFormView key={goal?.id ?? "new"} goal={goal} onDone={onClose} />}
      </DialogContent>
    </Dialog>
  );
}
