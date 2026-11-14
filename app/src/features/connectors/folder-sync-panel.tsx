import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open as pickFolder } from "@tauri-apps/plugin-dialog";
import { FolderOpen, RefreshCw } from "lucide-react";
import { useEffect } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { ImportProgressView } from "@/features/import/import-progress";
import { useImportRun } from "@/features/import/use-import-run";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDateTime } from "@/lib/format";

/** Last segment of a folder path ("…/RunGap/Documents/Export" → "Export"). */
function folderName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

/** Imports new TCX files from a watched folder (e.g. an iCloud export), as the iCloud sync did. */
export function FolderSyncPanel({
  onRunningChange,
}: {
  onRunningChange: (running: boolean) => void;
}) {
  const queryClient = useQueryClient();
  const { run, start } = useImportRun();
  const status = useQuery({ queryKey: ["folderSync"], queryFn: api.folderSyncStatus });
  const started = run.running || run.summary !== null || run.error !== null;
  const newFiles = status.data?.newFiles.length ?? 0;

  useEffect(() => onRunningChange(run.running), [run.running, onRunningChange]);
  // Recount new files after a sync.
  useEffect(() => {
    if (run.summary) void queryClient.invalidateQueries({ queryKey: ["folderSync"] });
  }, [run.summary, queryClient]);

  const chooseFolder = async () => {
    const folder = await pickFolder({
      directory: true,
      defaultPath: status.data?.folder ?? undefined,
    });
    if (typeof folder !== "string") return;
    try {
      queryClient.setQueryData(["folderSync"], await api.setSyncFolder(folder));
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <div className="grid gap-4">
      {/* min-w-0 stops long paths from widening the layout; the path wraps instead. */}
      <div className="grid min-w-0 gap-1 rounded-md border p-3 text-sm">
        <div className="flex min-w-0 items-start justify-between gap-3">
          <div className="grid min-w-0 gap-0.5">
            <span className="truncate font-medium">
              {status.data?.folder ? folderName(status.data.folder) : t("No folder chosen")}
            </span>
            {status.data?.folder && (
              <span className="text-xs break-all text-muted-foreground">{status.data.folder}</span>
            )}
          </div>
          <Button
            variant="outline"
            size="sm"
            className="shrink-0"
            disabled={run.running}
            onClick={chooseFolder}
          >
            <FolderOpen />
            {t("Choose…")}
          </Button>
        </div>
        <span className="text-muted-foreground">
          {t("Last sync: {{date}}", {
            date: status.data?.lastSync ? formatDateTime(status.data.lastSync) : t("never"),
          })}
          {status.data?.folder && ` · ${t("{{count}} new file", { count: newFiles })}`}
        </span>
      </div>
      {started && <ImportProgressView run={run} />}
      <div>
        <Button disabled={newFiles === 0 || run.running} onClick={() => start(api.syncFolder)}>
          <RefreshCw />
          {run.running
            ? t("Syncing…")
            : newFiles > 0
              ? t("Sync {{count}} file", { count: newFiles })
              : t("Sync")}
        </Button>
      </div>
    </div>
  );
}
