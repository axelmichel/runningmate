import { CircleAlert, CircleCheck, Copy } from "lucide-react";

import type { ImportProgress as Progress } from "@/bindings/ImportProgress";
import { Progress as ProgressBar } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { t } from "@/i18n";

import type { ImportRun } from "./use-import-run";

function Entry({ entry }: { entry: Progress }) {
  const { outcome } = entry;
  return (
    <li className="flex items-start gap-2 py-1">
      {outcome.status === "imported" && <CircleCheck className="mt-0.5 size-4 text-primary" />}
      {(outcome.status === "duplicate" || outcome.status === "similar") && (
        <Copy className="mt-0.5 size-4 text-muted-foreground" />
      )}
      {outcome.status === "failed" && <CircleAlert className="mt-0.5 size-4 text-destructive" />}
      <div className="min-w-0">
        <div className="truncate">{entry.file}</div>
        {outcome.status === "duplicate" && (
          <div className="text-xs text-muted-foreground">{t("Already imported")}</div>
        )}
        {outcome.status === "similar" && (
          <div className="text-xs text-muted-foreground">
            {t("Already there (same day and distance)")}
          </div>
        )}
        {outcome.status === "failed" && (
          <div className="text-xs break-words text-destructive">{outcome.error}</div>
        )}
      </div>
    </li>
  );
}

/** Progress bar, per-file log and summary of an import run. */
export function ImportProgressView({ run }: { run: ImportRun }) {
  const last = run.log.at(-1);
  const percent = last ? Math.round((last.done / last.total) * 100) : run.running ? 0 : 100;
  return (
    <div className="grid gap-3">
      <ProgressBar value={percent} />
      <div className="text-sm text-muted-foreground">
        {run.running &&
          (last
            ? t("{{done}} of {{total}} files", { done: last.done, total: last.total })
            : t("Starting…"))}
        {run.summary &&
          t("{{imported}} imported · {{duplicates}} already imported · {{failed}} failed", {
            imported: run.summary.imported,
            duplicates: run.summary.duplicates,
            failed: run.summary.failed,
          })}
        {run.error && <span className="text-destructive">{run.error}</span>}
      </div>
      {run.log.length > 0 && (
        <ScrollArea className="h-56 rounded-md border px-3 text-sm">
          <ul className="divide-y">
            {[...run.log].reverse().map((entry) => (
              <Entry key={`${entry.done}-${entry.file}`} entry={entry} />
            ))}
          </ul>
        </ScrollArea>
      )}
    </div>
  );
}
