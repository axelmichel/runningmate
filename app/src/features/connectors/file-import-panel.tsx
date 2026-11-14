import { open as pickFiles } from "@tauri-apps/plugin-dialog";
import { FileUp } from "lucide-react";
import { useEffect } from "react";

import { Button } from "@/components/ui/button";
import { ImportProgressView } from "@/features/import/import-progress";
import { useImportRun } from "@/features/import/use-import-run";
import { t } from "@/i18n";
import { api } from "@/lib/api";

/** Imports TCX files picked by hand (the former File → Import TCX). */
export function FileImportPanel({
  onRunningChange,
}: {
  onRunningChange: (running: boolean) => void;
}) {
  const { run, start } = useImportRun();
  const started = run.running || run.summary !== null || run.error !== null;

  useEffect(() => onRunningChange(run.running), [run.running, onRunningChange]);

  const choose = async () => {
    const files = await pickFiles({
      multiple: true,
      filters: [{ name: t("TCX files"), extensions: ["tcx"] }],
    });
    if (files && files.length > 0) await start(() => api.importFiles(files));
  };

  return (
    <div className="grid gap-4">
      <div>
        <Button disabled={run.running} onClick={() => void choose()}>
          <FileUp />
          {run.running ? t("Importing…") : t("Choose TCX files…")}
        </Button>
      </div>
      {started && <ImportProgressView run={run} />}
    </div>
  );
}
