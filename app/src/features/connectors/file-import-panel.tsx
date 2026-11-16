import { open as pickFiles } from "@tauri-apps/plugin-dialog";
import { FileUp } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { ImportProgressView } from "@/features/import/import-progress";
import { useImportRun } from "@/features/import/use-import-run";
import { t } from "@/i18n";
import { api } from "@/lib/api";

import { OPEN_RANGE, toDateRange, type RangeInput } from "./range";
import { RangeFields } from "./range-fields";

/** Imports TCX files picked by hand (the former File → Import TCX). */
export function FileImportPanel({
  onRunningChange,
}: {
  onRunningChange: (running: boolean) => void;
}) {
  const { run, start, cancel } = useImportRun();
  const [range, setRange] = useState<RangeInput>(OPEN_RANGE);
  const started = run.running || run.summary !== null || run.error !== null;

  useEffect(() => onRunningChange(run.running), [run.running, onRunningChange]);

  const choose = async () => {
    const files = await pickFiles({
      multiple: true,
      filters: [{ name: t("TCX / GPX files"), extensions: ["tcx", "gpx"] }],
    });
    if (files && files.length > 0) await start(() => api.importFiles(files, toDateRange(range)));
  };

  return (
    <div className="grid gap-4">
      <RangeFields
        id="files"
        value={range}
        onChange={setRange}
        disabled={run.running}
        hint={t("Optional: only files with activities in this period are imported. Newest first.")}
      />
      <div>
        <Button disabled={run.running} onClick={() => void choose()}>
          <FileUp />
          {run.running ? t("Importing…") : t("Choose TCX or GPX files…")}
        </Button>
      </div>
      {started && <ImportProgressView run={run} onCancel={cancel} />}
    </div>
  );
}
