import { useCallback, useMemo, useRef, useState } from "react";

import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { t } from "@/i18n";

import { GuardContext, type Editor } from "./use-unsaved-guard";

/**
 * Asks before leaving editors with unsaved changes: Save (then leave), Discard (leave) or
 * Cancel (stay). Editors register with `useUnsavedChanges`; ways out go through `useGuard`.
 */
export function UnsavedGuardProvider({ children }: { children: React.ReactNode }) {
  const editors = useRef(new Map<string, Editor>());
  const [pending, setPending] = useState<(() => void) | null>(null);
  const [saving, setSaving] = useState(false);

  const register = useCallback((id: string, editor: Editor | null) => {
    if (editor) editors.current.set(id, editor);
    else editors.current.delete(id);
  }, []);
  const guard = useCallback((action: () => void) => {
    const dirty = [...editors.current.values()].some((editor) => editor.dirty);
    if (dirty) setPending(() => action);
    else action();
  }, []);

  const leave = () => {
    const action = pending;
    setPending(null);
    action?.();
  };
  const saveAndLeave = async () => {
    setSaving(true);
    try {
      for (const editor of editors.current.values()) {
        if (editor.dirty) await editor.save();
      }
      leave();
    } catch {
      // The editor reports why; staying keeps the changes.
      setPending(null);
    } finally {
      setSaving(false);
    }
  };

  const asking = pending !== null;
  const value = useMemo(() => ({ register, guard, asking }), [register, guard, asking]);
  return (
    <GuardContext.Provider value={value}>
      {children}
      <AlertDialog
        open={pending !== null}
        onOpenChange={(open) => {
          if (!open && !saving) setPending(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Save your changes?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("You have changes that aren't saved yet. Without saving they are lost.")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={saving}>{t("Cancel")}</AlertDialogCancel>
            <Button variant="destructive" disabled={saving} onClick={leave}>
              {t("Discard")}
            </Button>
            <Button disabled={saving} onClick={() => void saveAndLeave()}>
              {saving ? t("Saving…") : t("Save")}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </GuardContext.Provider>
  );
}
