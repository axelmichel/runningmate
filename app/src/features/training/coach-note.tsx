import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, Sparkles } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { t } from "@/i18n";
import { api } from "@/lib/api";

/**
 * A coach's note on a plan week or, with `sessionId`, on a session and its run, written by the
 * local Ollama model. Only shown when AI is turned on in the settings; the plan never needs it.
 */
export function CoachNote({
  planId,
  week = null,
  sessionId = null,
}: {
  planId: number;
  week?: number | null;
  sessionId?: number | null;
}) {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const enabled = !!settings?.ai_enabled && !!settings.ai_model;
  const key = ["training", "note", planId, week, sessionId];
  const { data: note, isPending } = useQuery({
    queryKey: key,
    queryFn: () => api.getTrainingNote(planId, week, sessionId),
    enabled,
  });
  const generate = useMutation({
    mutationFn: () => api.generateTrainingNote(planId, week, sessionId),
    onSuccess: (saved) => queryClient.setQueryData(key, saved),
    onError: (error) =>
      toast.error(t("Could not write the note: {{error}}", { error: String(error) })),
  });
  if (!enabled || isPending) return null;

  return (
    <div className="grid gap-1.5 rounded-md bg-muted/50 p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="flex items-center gap-1.5 text-sm font-medium">
          <Sparkles className="size-4" />
          {sessionId === null ? t("This week") : t("Coach comment")}
        </span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={generate.isPending}
          onClick={() => generate.mutate()}
        >
          {generate.isPending && <Loader2 className="animate-spin" />}
          {note ? t("Rewrite") : sessionId === null ? t("Explain") : t("Comment")}
        </Button>
      </div>
      {note ? (
        <>
          <p className="text-sm whitespace-pre-wrap">{note.text}</p>
          <p className="text-xs text-muted-foreground">
            {t("Written by {{model}} on this computer. AI can be wrong.", { model: note.model })}
          </p>
        </>
      ) : (
        generate.isPending && <p className="text-sm text-muted-foreground">{t("Writing…")}</p>
      )}
    </div>
  );
}
