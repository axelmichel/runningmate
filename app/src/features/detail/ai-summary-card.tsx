import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, Sparkles } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { t } from "@/i18n";
import { api } from "@/lib/api";

/**
 * Summary of the activity against similar and recent ones, written by a local Ollama model.
 * Only shown when AI summaries are turned on in the settings.
 */
export function AiSummaryCard({ activityId }: { activityId: number }) {
  const queryClient = useQueryClient();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const enabled = !!settings?.ai_enabled && !!settings.ai_model;
  const { data: summary, isPending } = useQuery({
    queryKey: ["aiSummary", activityId],
    queryFn: () => api.getAiSummary(activityId),
    enabled,
  });
  const generate = useMutation({
    mutationFn: () => api.generateAiSummary(activityId),
    onSuccess: (saved) => queryClient.setQueryData(["aiSummary", activityId], saved),
    onError: (error) =>
      toast.error(t("Could not write the summary: {{error}}", { error: String(error) })),
  });
  if (!enabled || isPending) return null;

  return (
    <Card size="sm">
      <CardHeader className="flex flex-row items-center justify-between gap-2">
        <CardTitle className="flex items-center gap-1.5 text-sm">
          <Sparkles className="size-4" />
          {t("Summary")}
        </CardTitle>
        <Button
          variant="ghost"
          size="sm"
          disabled={generate.isPending}
          onClick={() => generate.mutate()}
        >
          {generate.isPending && <Loader2 className="animate-spin" />}
          {summary ? t("Rewrite") : t("Write summary")}
        </Button>
      </CardHeader>
      <CardContent className="grid gap-2">
        {summary ? (
          <>
            <p className="text-sm whitespace-pre-wrap">{summary.text}</p>
            <p className="text-xs text-muted-foreground">
              {t("Written by {{model}} on this computer. AI can be wrong.", {
                model: summary.model,
              })}
            </p>
          </>
        ) : (
          <p className="text-sm text-muted-foreground">
            {generate.isPending
              ? t("Comparing with your earlier activities…")
              : t("No summary yet.")}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
