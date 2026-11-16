import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { PenLine, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { FixEdit } from "@/bindings/FixEdit";
import type { PendingRoute } from "@/bindings/PendingRoute";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { RouteFixEditor } from "@/features/detail/route-fix-editor";
import { invalidateGoals } from "@/features/goals/goal-math";
import { invalidateSegments } from "@/features/segments/segment-math";
import { invalidateTraining } from "@/features/training/training-math";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDateTime, formatDuration, formatNumber } from "@/lib/format";

/** Under `activities`, so every import run refreshes it. */
const PENDING_KEY = ["activities", "pendingRoutes"];

/** Draws the whole route of a file without GPS, then imports it. */
function DrawRouteDialog({ file, onClose }: { file: PendingRoute | null; onClose: () => void }) {
  const queryClient = useQueryClient();
  const { data: source } = useQuery({
    queryKey: ["pendingFixSource", file?.fileId],
    queryFn: () => api.pendingFixSource(file!.fileId),
    enabled: file !== null,
  });
  const save = useMutation({
    mutationFn: (edits: FixEdit[]) => api.applyPendingFix(file!.fileId, edits),
    onSuccess: (outcome) => {
      void queryClient.invalidateQueries({ queryKey: ["activities"] });
      invalidateSegments(queryClient);
      invalidateGoals(queryClient);
      invalidateTraining(queryClient);
      if (outcome.status === "similar") {
        toast.info(t("Already there (same day and distance)"));
        return;
      }
      toast.success(
        outcome.status === "duplicate" ? t("Already imported") : t("Activity imported"),
      );
      onClose();
    },
    onError: (e) => toast.error(t("Could not save: {{error}}", { error: String(e) })),
  });

  return (
    <Dialog open={file !== null} onOpenChange={(open) => !open && !save.isPending && onClose()}>
      <DialogContent className="flex h-[85vh] max-w-6xl flex-col gap-4 sm:max-w-6xl">
        <DialogHeader>
          <DialogTitle>{t("Draw the route")}</DialogTitle>
          <DialogDescription>
            {t("This file has no GPS. Click the route you ran on the map, from start to finish.")}
          </DialogDescription>
        </DialogHeader>
        <div className="min-h-0 flex-1">
          {source ? (
            <RouteFixEditor
              source={source}
              saving={save.isPending}
              onSave={(edits) => save.mutate(edits)}
            />
          ) : (
            <Skeleton className="size-full" />
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

/** Imported files without GPS, which become activities once their route is drawn. */
export function PendingRoutes() {
  const queryClient = useQueryClient();
  const [drawing, setDrawing] = useState<PendingRoute | null>(null);
  const { data: files = [] } = useQuery({ queryKey: PENDING_KEY, queryFn: api.pendingRoutes });
  const discard = useMutation({
    mutationFn: (fileId: string) => api.discardPendingRoute(fileId),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: PENDING_KEY }),
    onError: (e) => toast.error(String(e)),
  });
  if (files.length === 0) return null;

  return (
    <section className="grid gap-2 rounded-lg border p-3">
      <div>
        <h3 className="text-sm font-semibold">{t("Waiting for a route")}</h3>
        <p className="text-xs text-muted-foreground">
          {t("These files have no GPS. Draw the route to import them.")}
        </p>
      </div>
      <ul className="divide-y text-sm">
        {files.map((file) => (
          <li key={file.fileId} className="flex items-center gap-3 py-1.5">
            <div className="min-w-0 flex-1">
              <div className="truncate">{file.fileId}</div>
              <div className="text-xs text-muted-foreground tabular-nums">
                {formatDateTime(file.start)} · {t(file.sport)} · {formatDuration(file.duration)}
                {file.distance != null && ` · ${formatNumber(file.distance, 2)} km`}
              </div>
            </div>
            <Button size="sm" onClick={() => setDrawing(file)}>
              <PenLine />
              {t("Draw route")}
            </Button>
            <Button
              size="icon"
              variant="ghost"
              aria-label={t("Remove")}
              disabled={discard.isPending}
              onClick={() => discard.mutate(file.fileId)}
            >
              <Trash2 />
            </Button>
          </li>
        ))}
      </ul>
      <DrawRouteDialog file={drawing} onClose={() => setDrawing(null)} />
    </section>
  );
}
