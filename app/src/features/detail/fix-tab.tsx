import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RotateCcw } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { FixEdit } from "@/bindings/FixEdit";
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
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { invalidateGoals } from "@/features/goals/goal-math";
import { invalidateSegments } from "@/features/segments/segment-math";
import { invalidateTraining } from "@/features/training/training-math";
import { t } from "@/i18n";
import { api } from "@/lib/api";

import { invalidateActivityTrack } from "./fix-math";
import { RouteFixEditor } from "./route-fix-editor";

/** The route fix editor for an activity, with a reset to the file it was imported from. */
export function FixTab({ activityId }: { activityId: number }) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const {
    data: source,
    dataUpdatedAt,
    isPending,
    error,
  } = useQuery({
    queryKey: ["fixSource", activityId],
    queryFn: () => api.activityFixSource(activityId),
  });

  const recomputed = (message: string) => {
    invalidateActivityTrack(queryClient, activityId);
    invalidateSegments(queryClient);
    invalidateGoals(queryClient);
    invalidateTraining(queryClient);
    toast.success(message);
  };
  const save = useMutation({
    mutationFn: (edits: FixEdit[]) => api.applyActivityFix(activityId, edits),
    onSuccess: () => recomputed(t("Route fixed, activity recomputed")),
    onError: (e) => toast.error(t("Could not save: {{error}}", { error: String(e) })),
  });
  const revert = useMutation({
    mutationFn: () => api.revertActivityFix(activityId),
    onSuccess: () => recomputed(t("Activity reset to its original file")),
    onError: (e) => toast.error(t("Could not reset: {{error}}", { error: String(e) })),
  });

  if (isPending) return <Skeleton className="h-full w-full" />;
  if (!source) {
    return (
      <p className="text-sm text-muted-foreground">
        {error ? String(error) : t("No archived TCX file for this activity.")}
      </p>
    );
  }
  return (
    <>
      <RouteFixEditor
        // A fresh editor once the saved fix comes back from the backend.
        key={dataUpdatedAt}
        source={source}
        saving={save.isPending}
        onSave={(edits) => save.mutate(edits)}
        hideMap={confirming}
        extraActions={
          source.hasOriginal && (
            <Button
              variant="outline"
              onClick={() => setConfirming(true)}
              disabled={revert.isPending}
            >
              <RotateCcw />
              {t("Reset to original")}
            </Button>
          )
        }
      />
      <AlertDialog open={confirming} onOpenChange={setConfirming}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Reset to the original file?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "All route fixes are dropped and the activity is recomputed from the file as it was imported.",
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => revert.mutate()}>
              {t("Reset")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
