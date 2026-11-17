import { createContext, useContext, useEffect, useId, useRef } from "react";

export type Editor = { dirty: boolean; save: () => Promise<unknown> };

type Guard = {
  register: (id: string, editor: Editor | null) => void;
  /** Runs `action` right away, or after asking when an editor has unsaved changes. */
  guard: (action: () => void) => void;
  /** Whether the question is showing (Leaflet maps paint above dialogs in WebKit). */
  asking: boolean;
};

export const GuardContext = createContext<Guard | null>(null);

/** Reports an editor's unsaved changes and how to save them; does nothing without a provider. */
export function useUnsavedChanges(dirty: boolean, save: () => Promise<unknown>) {
  const context = useContext(GuardContext);
  const id = useId();
  const saveRef = useRef(save);
  useEffect(() => {
    saveRef.current = save;
  });
  useEffect(() => {
    if (!context) return;
    context.register(id, { dirty, save: () => saveRef.current() });
    return () => context.register(id, null);
  }, [context, id, dirty]);
}

/** Wraps a way out of the editors, e.g. closing the dialog; runs directly without a provider. */
export function useGuard(): (action: () => void) => void {
  return useContext(GuardContext)?.guard ?? ((action) => action());
}

/** Whether the unsaved-changes question is showing. */
export function useGuardAsking(): boolean {
  return useContext(GuardContext)?.asking ?? false;
}
