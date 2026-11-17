import { useMutation, useQuery } from "@tanstack/react-query";
import { RotateCcw } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { FixEdit } from "@/bindings/FixEdit";
import type { ViewMode } from "@/bindings/ViewMode";
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
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";

import { useRecomputed } from "./fix-math";
import { RouteFixEditor } from "./route-fix-editor";
import { SampleFixEditor } from "./sample-fix-editor";
import type { SampleEdit } from "./sample-fix-math";
import { useGuard } from "./use-unsaved-guard";

type Part = "route" | "heartRate" | "steps";

const PARTS: { part: Part; label: string }[] = [
  { part: "route", label: tk("Route") },
  { part: "heartRate", label: tk("Heart rate") },
  { part: "steps", label: tk("Steps") },
];

/**
 * The route, heart rate and steps fix editors for an activity, with a reset to the file it was
 * imported from.
 */
export function FixTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const [confirming, setConfirming] = useState(false);
  const [part, setPart] = useState<Part>("route");
  const guard = useGuard();
  const {
    data: source,
    dataUpdatedAt,
    isPending,
    error,
  } = useQuery({
    queryKey: ["fixSource", activityId],
    queryFn: () => api.activityFixSource(activityId),
  });

  const recomputed = useRecomputed(activityId);
  const save = useMutation({
    mutationFn: (edits: FixEdit[]) => api.applyActivityFix(activityId, edits),
    onSuccess: () => recomputed(t("Route fixed, activity recomputed")),
    onError: (e) => toast.error(t("Could not save: {{error}}", { error: String(e) })),
  });
  const saveSamples = useMutation({
    mutationFn: (edits: SampleEdit[]) => api.applySampleFix(activityId, edits),
    onSuccess: () => recomputed(t("Values fixed, activity recomputed")),
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
  const reset = source.hasOriginal && (
    <Button variant="outline" onClick={() => setConfirming(true)} disabled={revert.isPending}>
      <RotateCcw />
      {t("Reset to original")}
    </Button>
  );
  // Rides have no steps.
  const parts = PARTS.filter(({ part }) => part !== "steps" || view !== "Cycling");
  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <Tabs value={part} onValueChange={(value) => guard(() => setPart(value as Part))}>
        <TabsList>
          {parts.map(({ part, label }) => (
            <TabsTrigger key={part} value={part}>
              {t(label)}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      <div className="min-h-0 flex-1">
        {part === "route" ? (
          <RouteFixEditor
            // A fresh editor once the saved fix comes back from the backend.
            key={dataUpdatedAt}
            source={source}
            saving={save.isPending}
            onSave={(edits) => save.mutateAsync(edits)}
            hideMap={confirming}
            extraActions={reset}
          />
        ) : (
          <SampleFixEditor
            key={`${part}-${dataUpdatedAt}`}
            source={source}
            channel={part}
            saving={saveSamples.isPending}
            onSave={(edits) => saveSamples.mutateAsync(edits)}
            extraActions={reset}
          />
        )}
      </div>
      <AlertDialog open={confirming} onOpenChange={setConfirming}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Reset to the original file?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "All fixes of route, heart rate and steps are dropped and the activity is recomputed from the file as it was imported.",
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
    </div>
  );
}
