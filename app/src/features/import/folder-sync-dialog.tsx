import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open as pickFolder } from "@tauri-apps/plugin-dialog";
import { FolderOpen, RefreshCw } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api } from "@/lib/api";
import { formatDateTime } from "@/lib/format";

import { ImportProgressView } from "./import-progress";
import { useImportRun } from "./use-import-run";

type FolderSyncDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/** Imports new TCX files from a watched folder (e.g. an iCloud export), as the iCloud sync did. */
export function FolderSyncDialog({ open, onOpenChange }: FolderSyncDialogProps) {
  const queryClient = useQueryClient();
  const { run, start, reset } = useImportRun();
  const status = useQuery({
    queryKey: ["folderSync"],
    queryFn: api.folderSyncStatus,
    enabled: open,
  });
  const started = run.running || run.summary !== null || run.error !== null;
  const newFiles = status.data?.newFiles.length ?? 0;

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

  const close = (next: boolean) => {
    if (run.running) return;
    onOpenChange(next);
    if (!next) {
      reset();
      queryClient.invalidateQueries({ queryKey: ["folderSync"] });
    }
  };

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent showCloseButton={!run.running}>
        <DialogHeader>
          <DialogTitle>Sync folder</DialogTitle>
          <DialogDescription>
            Imports TCX files added or changed in the folder since the last sync.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-1 rounded-md border p-3 text-sm">
          <div className="flex items-center justify-between gap-3">
            <span className="min-w-0 truncate font-medium" title={status.data?.folder ?? ""}>
              {status.data?.folder ?? "No folder chosen"}
            </span>
            <Button variant="outline" size="sm" disabled={run.running} onClick={chooseFolder}>
              <FolderOpen />
              Choose…
            </Button>
          </div>
          <span className="text-muted-foreground">
            Last sync: {status.data?.lastSync ? formatDateTime(status.data.lastSync) : "never"}
            {status.data?.folder &&
              !started &&
              ` · ${newFiles} new file${newFiles === 1 ? "" : "s"}`}
          </span>
        </div>

        {started && <ImportProgressView run={run} />}

        <DialogFooter>
          {started ? (
            <Button disabled={run.running} onClick={() => close(false)}>
              {run.running ? "Syncing…" : "Done"}
            </Button>
          ) : (
            <Button disabled={newFiles === 0} onClick={() => start(api.syncFolder)}>
              <RefreshCw />
              Sync {newFiles > 0 ? `${newFiles} file${newFiles === 1 ? "" : "s"}` : ""}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
