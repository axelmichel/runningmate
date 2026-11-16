import { useCallback, useEffect, useState } from "react";

/** Decoded images by key for the whole session; `null` when loading failed. */
const loaded = new Map<string, ImageBitmap | null>();
const loading = new Map<string, Promise<void>>();
const listeners = new Set<() => void>();
let scheduled = false;

/** Tells the editors once per frame, however many images arrived in it. */
function notify() {
  if (scheduled) return;
  scheduled = true;
  requestAnimationFrame(() => {
    scheduled = false;
    listeners.forEach((listener) => listener());
  });
}

/**
 * Looks up images for the share canvas, loading missing ones in the background: `get` returns
 * `undefined` until an image is there, and `version` changes when one arrives.
 */
export function useBitmaps() {
  const [version, setVersion] = useState(0);
  useEffect(() => {
    const listener = () => setVersion((v) => v + 1);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, []);
  const bitmap = useCallback(
    (key: string, load: () => Promise<ArrayBuffer>): ImageBitmap | undefined => {
      const image = loaded.get(key);
      if (image !== undefined) return image ?? undefined;
      if (!loading.has(key)) {
        loading.set(
          key,
          load()
            .then((bytes) => createImageBitmap(new Blob([bytes])))
            .then(
              (bitmap) => void loaded.set(key, bitmap),
              () => void loaded.set(key, null),
            )
            .finally(() => {
              loading.delete(key);
              notify();
            }),
        );
      }
      return undefined;
    },
    [],
  );
  return { bitmap, version };
}
