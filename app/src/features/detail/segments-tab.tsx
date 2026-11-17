import { useMutation, useQuery } from "@tanstack/react-query";
import { Pencil } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { SegmentRow } from "@/bindings/SegmentRow";
import type { ViewMode } from "@/bindings/ViewMode";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDuration, formatNumber, formatPace } from "@/lib/format";

import { useRecomputed } from "./fix-math";
import { splitEdits, type SplitChange } from "./sample-fix-math";
import { useUnsavedChanges } from "./use-unsaved-guard";

type Field = keyof SplitChange;
/** Entered texts per split index. */
type Draft = Record<number, Partial<Record<Field, string>>>;

/** Shown (and entered) value of a split: bpm, or steps per minute of both feet. */
function shown(segment: SegmentRow, field: Field): number | null {
  if (field === "heartRate") return segment.heartRate || null;
  return segment.steps ? segment.steps * 2 : null;
}

/** The splits whose entered values differ from the shown ones. */
function changes(segments: SegmentRow[], draft: Draft): Record<number, SplitChange> {
  const out: Record<number, SplitChange> = {};
  for (const segment of segments) {
    for (const field of ["heartRate", "steps"] as const) {
      const text = draft[segment.index]?.[field];
      const current = shown(segment, field);
      if (text === undefined || current === null) continue;
      const value = Number(text);
      if (text.trim() === "" || !(value > 0) || value === Math.round(current)) continue;
      out[segment.index] = { ...out[segment.index], [field]: value };
    }
  }
  return out;
}

/**
 * Per-km (per 5 km for rides) splits, the `activity_details` rows. Heart rate and steps can be
 * corrected per split: the split's samples are shifted to the entered average and the activity
 * recomputed, so the change also holds after a re-import.
 */
export function SegmentsTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const { data: segments = [] } = useQuery({
    queryKey: ["segments", activityId],
    queryFn: () => api.activitySegments(activityId),
  });
  const [draft, setDraft] = useState<Draft | null>(null);
  const recomputed = useRecomputed(activityId);
  const changed = draft ? changes(segments, draft) : {};
  const edits = splitEdits(segments, changed);
  const save = useMutation({
    mutationFn: () => api.applySampleFix(activityId, edits),
    onSuccess: () => {
      setDraft(null);
      recomputed(t("Splits corrected, activity recomputed"));
    },
    onError: (e) => toast.error(t("Could not save: {{error}}", { error: String(e) })),
  });
  useUnsavedChanges(edits.length > 0, () => save.mutateAsync());

  const steps = view !== "Cycling";
  const editable = segments.some((s) => s.start != null && (s.heartRate || s.steps));
  const head = "text-right";
  const cell = "text-right tabular-nums";
  const value = (segment: SegmentRow, field: Field) => {
    const current = shown(segment, field);
    if (!draft || current === null || segment.start == null) return formatNumber(current);
    const text = draft[segment.index]?.[field] ?? String(Math.round(current));
    return (
      <Input
        type="number"
        min={1}
        aria-label={`${field === "heartRate" ? t("HR (bpm)") : t("Cadence (spm)")} ${segment.index + 1}`}
        className="ml-auto h-7 w-20 text-right tabular-nums"
        value={text}
        onChange={(e) =>
          setDraft({
            ...draft,
            [segment.index]: { ...draft[segment.index], [field]: e.target.value },
          })
        }
      />
    );
  };
  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      {editable && (
        <div className="flex items-center justify-end gap-2">
          {draft ? (
            <>
              <span className="mr-auto text-xs text-muted-foreground">
                {t("Enter the average a split should have; its samples are shifted to match.")}
              </span>
              <Button variant="ghost" size="sm" onClick={() => setDraft(null)}>
                {t("Cancel")}
              </Button>
              <Button
                size="sm"
                disabled={edits.length === 0 || save.isPending}
                onClick={() => save.mutate()}
              >
                {save.isPending ? t("Saving…") : t("Save and recompute")}
              </Button>
            </>
          ) : (
            <Button variant="outline" size="sm" onClick={() => setDraft({})}>
              <Pencil />
              {steps ? t("Correct heart rate and steps") : t("Correct heart rate")}
            </Button>
          )}
        </div>
      )}
      <div className="scrollbar-visible min-h-0 flex-1 overflow-auto rounded-lg border">
        <Table>
          <TableHeader className="sticky top-0 bg-background">
            <TableRow>
              <TableHead>#</TableHead>
              <TableHead className={head}>{t("Distance (km)")}</TableHead>
              <TableHead className={head}>{t("Time")}</TableHead>
              <TableHead className={head}>{t("Pace (/km)")}</TableHead>
              {steps && (
                <TableHead className={head} title={t("Grade-adjusted pace")}>
                  {t("GAP (/km)")}
                </TableHead>
              )}
              <TableHead className={head}>{t("Speed (km/h)")}</TableHead>
              <TableHead className={head}>{t("HR (bpm)")}</TableHead>
              <TableHead className={head}>{t("Power (W)")}</TableHead>
              {steps && <TableHead className={head}>{t("Cadence (spm)")}</TableHead>}
              <TableHead className={head}>{t("Elevation (m)")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {segments.map((s) => (
              <TableRow key={s.index}>
                <TableCell className="text-muted-foreground">{s.index + 1}</TableCell>
                <TableCell className={cell}>{formatNumber(s.distance, 2)}</TableCell>
                <TableCell className={cell}>{formatDuration(s.duration)}</TableCell>
                <TableCell className={cell}>{formatPace(s.pace)}</TableCell>
                {steps && <TableCell className={cell}>{formatPace(s.gap)}</TableCell>}
                <TableCell className={cell}>
                  {formatNumber(s.speed == null ? null : s.speed * 3.6, 1)}
                </TableCell>
                <TableCell className={cell}>{value(s, "heartRate")}</TableCell>
                <TableCell className={cell}>{formatNumber(s.power || null)}</TableCell>
                {steps && <TableCell className={cell}>{value(s, "steps")}</TableCell>}
                <TableCell className={cell}>{formatNumber(s.elevationGain)}</TableCell>
              </TableRow>
            ))}
            {segments.length === 0 && (
              <TableRow>
                <TableCell
                  colSpan={steps ? 10 : 8}
                  className="py-10 text-center text-muted-foreground"
                >
                  {t("No splits for this activity.")}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}
