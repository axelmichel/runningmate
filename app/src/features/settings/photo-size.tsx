import { useIsMutating, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useState } from "react";
import { toast } from "sonner";

import type { PhotoSize } from "@/bindings/PhotoSize";
import type { ShrinkProgress } from "@/bindings/ShrinkProgress";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/format";

import { formatBytes } from "./validation";

/** The presets with what they are good for; sizes measured on 12-megapixel phone photos. */
const PHOTO_SIZES: [PhotoSize, string, string][] = [
  [
    "recommended",
    tk("Recommended · 2560 px, quality 85"),
    tk(
      "Sharp full screen on a Retina laptop, good for the gallery and for sharing. About 0.7–2.5 MB per photo.",
    ),
  ],
  [
    "large",
    tk("Large · 3840 px, quality 90"),
    tk("Sharp full screen on a 4K or 5K display. About 2–5.5 MB per photo."),
  ],
  [
    "small",
    tk("Small · 1920 px, quality 80"),
    tk(
      "Enough for a Full HD screen and the gallery, takes the least space. About 0.4–1.3 MB per photo.",
    ),
  ],
  ["original", tk("Original"), tk("Kept exactly as they are. About 2–8 MB per photo.")],
];

/** How far photos copied into RunningMate are scaled down (`photo_size` in settings.json). */
export function PhotoSizeSetting({
  value,
  onChange,
}: {
  value: PhotoSize;
  onChange: (size: PhotoSize) => void;
}) {
  return (
    <div className="grid gap-2">
      <Label>{t("Photo size")}</Label>
      <RadioGroup value={value} onValueChange={(size) => onChange(size as PhotoSize)}>
        {PHOTO_SIZES.map(([size, label, hint]) => (
          <div key={size} className="flex items-start gap-2">
            <RadioGroupItem value={size} id={`photo-size-${size}`} className="mt-0.5" />
            <Label htmlFor={`photo-size-${size}`} className="grid gap-0.5 font-normal">
              <span>{t(label)}</span>
              <span className="text-xs text-muted-foreground">{t(hint)}</span>
            </Label>
          </div>
        ))}
      </RadioGroup>
      <p className="text-xs text-muted-foreground">
        {t(
          "Photos copied into RunningMate are scaled down to this size and saved as JPEG. Linked photos and videos are never changed.",
        )}
      </p>
    </div>
  );
}

/** Replaces stored copies larger than the photo size by smaller ones, with progress. */
export function ShrinkPhotos({ size }: { size: PhotoSize }) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const [progress, setProgress] = useState<ShrinkProgress | null>(null);
  const preview = useQuery({
    queryKey: ["photosToShrink", size],
    queryFn: api.photosToShrink,
    enabled: size !== "original",
  });
  const shrink = useMutation({
    mutationKey: ["shrinkPhotos"],
    mutationFn: async () => {
      setProgress(null);
      const stop = await listen<ShrinkProgress>("media://shrink", (event) =>
        setProgress(event.payload),
      );
      try {
        return await api.shrinkPhotos();
      } finally {
        stop();
      }
    },
    onSuccess: (summary) => {
      toast.success(
        t("{{count}} photos shrunk, {{size}} saved", {
          count: summary.shrunk,
          photos: formatNumber(summary.shrunk),
          size: formatBytes(summary.saved),
        }),
      );
      if (summary.failed > 0) {
        toast.error(
          t("{{count}} photos couldn't be shrunk and were left as they are", {
            count: summary.failed,
            photos: formatNumber(summary.failed),
          }),
        );
      }
    },
    onError: (error) =>
      toast.error(t("Could not shrink the photos: {{error}}", { error: String(error) })),
    onSettled: () => {
      setProgress(null);
      for (const key of ["storage", "photosToShrink", "gallery", "recentPhotos", "media"]) {
        void queryClient.invalidateQueries({ queryKey: [key] });
      }
    },
  });
  // Also true when the run was started before the settings were closed and opened again.
  const running = useIsMutating({ mutationKey: ["shrinkPhotos"] }) > 0;

  if (size === "original") return null;
  const count = preview.data?.photos ?? 0;

  return (
    <div className="grid gap-2">
      <Label>{t("Stored photos")}</Label>
      {running ? (
        <div className="grid gap-1.5">
          <Progress value={progress ? (progress.done / progress.total) * 100 : 0} />
          <p className="text-xs text-muted-foreground">
            {progress
              ? t("{{done}} of {{total}} photos · {{size}} saved", {
                  done: formatNumber(progress.done),
                  total: formatNumber(progress.total),
                  size: formatBytes(progress.saved),
                })
              : t("Shrinking photos…")}
          </p>
        </div>
      ) : preview.data ? (
        <p className="text-sm text-muted-foreground">
          {count === 0
            ? t("All stored photos fit this size.")
            : t("{{count}} stored photos are larger than this size ({{size}} together).", {
                count,
                photos: formatNumber(count),
                size: formatBytes(preview.data.bytes),
              })}
        </p>
      ) : null}
      <div>
        <Button
          variant="outline"
          size="sm"
          disabled={running || count === 0}
          onClick={() => setConfirming(true)}
        >
          {t("Shrink stored photos…")}
        </Button>
      </div>
      <AlertDialog open={confirming} onOpenChange={setConfirming}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Shrink stored photos?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "{{count}} photos copied into RunningMate are replaced by smaller versions at the chosen size. Linked photos and their originals stay untouched. This can't be undone.",
                { count, photos: formatNumber(count) },
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={() => shrink.mutate()}>
              {t("Shrink photos")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
