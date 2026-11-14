import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

import { t } from "@/i18n";

import { ImportProgressView } from "./import-progress";
import type { ImportRun } from "./use-import-run";

type ImportDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  run: ImportRun;
};

/** Shows the progress of a TCX file import started from the menu or sidebar. */
export function ImportDialog({ open, onOpenChange, run }: ImportDialogProps) {
  return (
    <Dialog open={open} onOpenChange={(next) => !run.running && onOpenChange(next)}>
      <DialogContent showCloseButton={!run.running}>
        <DialogHeader>
          <DialogTitle>{t("Import TCX files")}</DialogTitle>
          <DialogDescription>
            {t("Files are archived in the imports folder; the originals stay where they are.")}
          </DialogDescription>
        </DialogHeader>
        <ImportProgressView run={run} />
        <DialogFooter>
          <Button disabled={run.running} onClick={() => onOpenChange(false)}>
            {run.running ? t("Importing…") : t("Done")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
