import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useState } from "react";

import type { ImportProgress } from "@/bindings/ImportProgress";
import type { ImportSummary } from "@/bindings/ImportSummary";
import type { ReadingProgress } from "@/bindings/ReadingProgress";
import { invalidateGoals } from "@/features/goals/goal-math";
import { invalidateSegments } from "@/features/segments/segment-math";
import { api } from "@/lib/api";

export type ImportRun = {
  running: boolean;
  /** Files being read before importing (iCloud Drive may download them first). */
  reading: ReadingProgress | null;
  log: ImportProgress[];
  summary: ImportSummary | null;
  error: string | null;
  /** Cancel was requested; the run stops after the current file. */
  cancelling: boolean;
};

const IDLE: ImportRun = {
  running: false,
  reading: null,
  log: [],
  summary: null,
  error: null,
  cancelling: false,
};

/** Runs an import or sync command and collects its `import://reading` and `import://progress` events. */
export function useImportRun() {
  const queryClient = useQueryClient();
  const [run, setRun] = useState<ImportRun>(IDLE);

  const start = useCallback(
    async (command: () => Promise<ImportSummary>) => {
      setRun({ ...IDLE, running: true });
      const stopReading = await listen<ReadingProgress>("import://reading", (event) =>
        setRun((current) => ({ ...current, reading: event.payload })),
      );
      const stopProgress = await listen<ImportProgress>("import://progress", (event) =>
        setRun((current) => ({ ...current, log: [...current.log, event.payload] })),
      );
      try {
        const summary = await command();
        setRun((current) => ({ ...current, running: false, summary }));
      } catch (error) {
        setRun((current) => ({ ...current, running: false, error: String(error) }));
      } finally {
        stopReading();
        stopProgress();
        queryClient.invalidateQueries({ queryKey: ["activities"] });
        invalidateSegments(queryClient);
        invalidateGoals(queryClient);
      }
    },
    [queryClient],
  );

  const cancel = useCallback(() => {
    setRun((current) => ({ ...current, cancelling: true }));
    void api.cancelImport();
  }, []);

  const reset = useCallback(() => setRun(IDLE), []);

  return { run, start, cancel, reset };
}
