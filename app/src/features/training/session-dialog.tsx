import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link2, Link2Off, SkipForward, Trash2, Undo2 } from "lucide-react";
import { useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { SessionInput } from "@/bindings/SessionInput";
import type { SessionKind } from "@/bindings/SessionKind";
import type { SessionStatus } from "@/bindings/SessionStatus";
import type { Structure } from "@/bindings/Structure";
import { Button } from "@/components/ui/button";
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
import { Textarea } from "@/components/ui/textarea";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDateTime, formatNumber, parseDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

import { CoachNote } from "./coach-note";
import {
  KINDS,
  KIND_LABELS,
  VERDICT_COLORS,
  checkText,
  formatPaceSeconds,
  invalidateTraining,
} from "./training-math";

function Field({
  id,
  label,
  error,
  children,
}: {
  id: string;
  label: string;
  error?: string;
  children: ReactNode;
}) {
  return (
    <div className="grid content-start gap-1.5">
      <Label htmlFor={id}>{label}</Label>
      {children}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}

type SessionForm = {
  date: string;
  kind: SessionKind;
  distance: string;
  paceFast: string;
  paceSlow: string;
  reps: string;
  /** m for distance reps, min for timed ones. */
  each: string;
  timed: boolean;
  rest: string;
  note: string;
};

const hasWork = (kind: SessionKind) => kind === "tempo" || kind === "intervals";

function sessionForm(status: SessionStatus | null, date: string): SessionForm {
  const session = status?.session;
  const structure = session?.structure;
  return {
    date: session?.date ?? date,
    kind: session?.kind ?? "easy",
    distance: session ? String(session.distance) : "",
    paceFast: session?.pace ? formatPaceSeconds(session.pace.fast) : "",
    paceSlow: session?.pace ? formatPaceSeconds(session.pace.slow) : "",
    reps: String(structure?.reps ?? 4),
    each:
      structure?.seconds != null
        ? String(structure.seconds / 60)
        : String(structure?.metres ?? 1000),
    timed: structure?.seconds != null,
    rest: String(structure?.rest ?? 120),
    note: session?.note ?? "",
  };
}

/** The input, or the error of the first field that is wrong. */
function sessionInput(form: SessionForm): SessionInput | string {
  const distance = Number(form.distance.replace(",", "."));
  if (!(distance > 0)) return t("Enter a distance above zero");
  const fast = form.paceFast.trim() ? parseDuration(form.paceFast) : null;
  const slow = form.paceSlow.trim() ? parseDuration(form.paceSlow) : null;
  if ((form.paceFast.trim() && !fast) || (form.paceSlow.trim() && !slow)) {
    return t("Enter paces as M:SS");
  }
  const pace = fast && slow ? { fast: Math.min(fast, slow), slow: Math.max(fast, slow) } : null;
  let structure: Structure | null = null;
  if (hasWork(form.kind)) {
    const reps = Number(form.reps);
    const each = Number(form.each.replace(",", "."));
    const rest = Number(form.rest);
    if (!(Number.isInteger(reps) && reps > 0 && each > 0 && rest >= 0)) {
      return t("Enter the repetitions as numbers");
    }
    structure = form.timed
      ? { reps, seconds: Math.round(each * 60), rest }
      : { reps, metres: Math.round(each), rest };
  }
  return { date: form.date, kind: form.kind, distance, pace, structure, note: form.note };
}

/** Runs around the session to link one by hand. */
function LinkRuns({ status, onDone }: { status: SessionStatus; onDone: () => void }) {
  const queryClient = useQueryClient();
  const { session, run } = status;
  const { data: runs = [] } = useQuery({
    queryKey: ["training", "candidates", session.id],
    queryFn: () => api.trainingLinkCandidates(session.id),
  });
  const link = useMutation({
    mutationFn: ({ activityId, unlinked }: { activityId: number | null; unlinked: boolean }) =>
      api.linkTrainingSession(session.id, activityId, unlinked),
    onSuccess: () => {
      invalidateTraining(queryClient);
      onDone();
    },
    onError: (error) => toast.error(t("Could not save: {{error}}", { error: String(error) })),
  });

  return (
    <div className="grid gap-2">
      <div className="text-sm font-medium">{t("Run")}</div>
      {run ? (
        <div className={cn("text-xs", VERDICT_COLORS[status.check.verdict])}>
          {checkText(status)}
        </div>
      ) : (
        <div className="text-xs text-muted-foreground">
          {session.unlinked
            ? t("No run is matched automatically.")
            : t(
                "No run matched yet. Runs on the day or a day before or after are matched automatically.",
              )}
        </div>
      )}
      {runs.length > 0 && (
        <div className="grid gap-1">
          {runs.map((candidate) => {
            const current = candidate.id === run?.id;
            return (
              <button
                key={candidate.id}
                type="button"
                disabled={current || link.isPending}
                className={cn(
                  "flex items-center gap-2 rounded-md border px-2 py-1.5 text-left text-xs transition-colors hover:bg-muted/50 disabled:opacity-100",
                  current && "border-primary bg-primary/5",
                )}
                onClick={() => link.mutate({ activityId: candidate.id, unlinked: false })}
              >
                <Link2 className="size-3.5 shrink-0" />
                <span className="min-w-0 flex-1 truncate">
                  {candidate.title || t("Activity")} · {formatDateTime(candidate.date)}
                </span>
                <span className="tabular-nums">
                  {formatNumber(candidate.distance, 1)} km · {formatPaceSeconds(candidate.pace)} /km
                </span>
              </button>
            );
          })}
        </div>
      )}
      <div className="flex flex-wrap gap-2">
        {(session.activityId !== null || session.unlinked) && (
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={link.isPending}
            onClick={() => link.mutate({ activityId: null, unlinked: false })}
          >
            <Undo2 />
            {t("Match automatically")}
          </Button>
        )}
        {!session.unlinked && (
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={link.isPending}
            onClick={() => link.mutate({ activityId: null, unlinked: true })}
          >
            <Link2Off />
            {t("No run for this session")}
          </Button>
        )}
      </div>
    </div>
  );
}

function SessionFormView({
  status,
  planId,
  date,
  onDone,
}: {
  status: SessionStatus | null;
  planId: number;
  date: string;
  onDone: () => void;
}) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState(() => sessionForm(status, date));
  const [error, setError] = useState<string | null>(null);
  const set = (change: Partial<SessionForm>) => setForm((current) => ({ ...current, ...change }));
  const session = status?.session;
  const onError = (e: unknown) => toast.error(t("Could not save: {{error}}", { error: String(e) }));
  const done = () => {
    invalidateTraining(queryClient);
    onDone();
  };

  const save = useMutation({
    mutationFn: (input: SessionInput) =>
      session
        ? api.updateTrainingSession(session.id, input)
        : api.addTrainingSession(planId, input),
    onSuccess: () => {
      toast.success(t("Session saved"));
      done();
    },
    onError,
  });
  const skip = useMutation({
    mutationFn: () => api.skipTrainingSession(session!.id, !session!.skipped),
    onSuccess: done,
    onError,
  });
  const remove = useMutation({
    mutationFn: () => api.deleteTrainingSession(session!.id),
    onSuccess: done,
    onError,
  });

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        const input = sessionInput(form);
        if (typeof input === "string") setError(input);
        else save.mutate(input);
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        <Field id="session-kind" label={t("Kind")}>
          <Select value={form.kind} onValueChange={(kind) => set({ kind: kind as SessionKind })}>
            <SelectTrigger id="session-kind" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {KINDS.map((kind) => (
                <SelectItem key={kind} value={kind}>
                  {t(KIND_LABELS[kind])}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>
        <Field id="session-date" label={t("Date")}>
          <Input
            id="session-date"
            type="date"
            value={form.date}
            onChange={(event) => set({ date: event.target.value })}
          />
        </Field>
        <Field id="session-distance" label={`${t("Distance")} (km)`}>
          <Input
            id="session-distance"
            inputMode="decimal"
            value={form.distance}
            onChange={(event) => set({ distance: event.target.value })}
          />
        </Field>
        <Field id="session-pace" label={`${t("Pace")} (M:SS /km)`}>
          <div className="flex items-center gap-2">
            <Input
              id="session-pace"
              placeholder="5:10"
              value={form.paceFast}
              onChange={(event) => set({ paceFast: event.target.value })}
            />
            –
            <Input
              aria-label={t("Slowest pace")}
              placeholder="5:40"
              value={form.paceSlow}
              onChange={(event) => set({ paceSlow: event.target.value })}
            />
          </div>
        </Field>
        {hasWork(form.kind) && (
          <div className="grid grid-cols-[1fr_1fr_auto_1fr] items-end gap-2 sm:col-span-2">
            <Field id="session-reps" label={t("Repetitions")}>
              <Input
                id="session-reps"
                inputMode="numeric"
                value={form.reps}
                onChange={(event) => set({ reps: event.target.value })}
              />
            </Field>
            <Field id="session-each" label={form.timed ? t("Minutes each") : t("Metres each")}>
              <Input
                id="session-each"
                inputMode="decimal"
                value={form.each}
                onChange={(event) => set({ each: event.target.value })}
              />
            </Field>
            <Button
              type="button"
              variant="outline"
              onClick={() => set({ timed: !form.timed, each: form.timed ? "1000" : "10" })}
            >
              {form.timed ? "min" : "m"}
            </Button>
            <Field id="session-rest" label={t("Rest (s)")}>
              <Input
                id="session-rest"
                inputMode="numeric"
                value={form.rest}
                onChange={(event) => set({ rest: event.target.value })}
              />
            </Field>
          </div>
        )}
        <div className="sm:col-span-2">
          <Field id="session-note" label={t("Note")}>
            <Textarea
              id="session-note"
              rows={2}
              value={form.note}
              onChange={(event) => set({ note: event.target.value })}
            />
          </Field>
        </div>
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
      {status && <LinkRuns status={status} onDone={onDone} />}
      {status?.run && <CoachNote planId={planId} sessionId={status.session.id} />}
      <DialogFooter className="sm:justify-between">
        <div className="flex gap-2">
          {session && (
            <>
              <Button type="button" variant="outline" onClick={() => skip.mutate()}>
                {session.skipped ? <Undo2 /> : <SkipForward />}
                {session.skipped ? t("Not skipped") : t("Skip")}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                title={t("Delete")}
                aria-label={t("Delete")}
                onClick={() => remove.mutate()}
              >
                <Trash2 />
              </Button>
            </>
          )}
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="outline" onClick={onDone}>
            {t("Cancel")}
          </Button>
          <Button type="submit" disabled={save.isPending}>
            {t("Save")}
          </Button>
        </div>
      </DialogFooter>
    </form>
  );
}

/** Edits a session, links a run to it, or with `status` null adds one on `date`. */
export function SessionDialog({
  open,
  status,
  planId,
  date,
  onClose,
}: {
  open: boolean;
  status: SessionStatus | null;
  planId: number;
  date: string;
  onClose: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>
            {status ? t(KIND_LABELS[status.session.kind]) : t("New session")}
          </DialogTitle>
          <DialogDescription>
            {t("Changed sessions stay as they are when the plan is adapted.")}
          </DialogDescription>
        </DialogHeader>
        {open && (
          <SessionFormView
            key={status?.session.id ?? `new-${date}`}
            status={status}
            planId={planId}
            date={date}
            onDone={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
