import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useState } from "react";

import type { ImportProgress } from "@/bindings/ImportProgress";
import type { ImportSummary } from "@/bindings/ImportSummary";

export type ImportRun = {
  running: boolean;
  log: ImportProgress[];
  summary: ImportSummary | null;
  error: string | null;
};

const IDLE: ImportRun = { running: false, log: [], summary: null, error: null };

/** Runs an import command and collects its `import://progress` events. */
export function useImportRun() {
  const queryClient = useQueryClient();
  const [run, setRun] = useState<ImportRun>(IDLE);

  const start = useCallback(
    async (command: () => Promise<ImportSummary>) => {
      setRun({ ...IDLE, running: true });
      const unlisten = await listen<ImportProgress>("import://progress", (event) =>
        setRun((current) => ({ ...current, log: [...current.log, event.payload] })),
      );
      try {
        const summary = await command();
        setRun((current) => ({ ...current, running: false, summary }));
      } catch (error) {
        setRun((current) => ({ ...current, running: false, error: String(error) }));
      } finally {
        unlisten();
        queryClient.invalidateQueries({ queryKey: ["activities"] });
      }
    },
    [queryClient],
  );

  const reset = useCallback(() => setRun(IDLE), []);

  return { run, start, reset };
}
