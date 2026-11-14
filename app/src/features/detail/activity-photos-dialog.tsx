import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, ImageOff, Loader2, Play } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { LibraryAsset } from "@/bindings/LibraryAsset";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDuration, formatTime } from "@/lib/format";
import { cn } from "@/lib/utils";

const PRIVACY_SETTINGS = "x-apple.systempreferences:com.apple.preference.security?Privacy_Photos";

/** One library item: a lazily rendered preview with its time and a tick box. */
function AssetTile({
  asset,
  selected,
  onToggle,
}: {
  asset: LibraryAsset;
  selected: boolean;
  onToggle: () => void;
}) {
  const thumbnail = useQuery({
    queryKey: ["libraryThumbnail", asset.id],
    queryFn: () => api.libraryPhotoThumbnail(asset.id),
    staleTime: Infinity,
    retry: false,
  });
  return (
    <button
      type="button"
      disabled={asset.added}
      onClick={onToggle}
      aria-pressed={selected}
      className={cn(
        "group relative aspect-square overflow-hidden rounded-md border bg-muted text-left",
        selected && "ring-2 ring-primary",
        asset.added && "cursor-default opacity-60",
      )}
    >
      {thumbnail.data ? (
        <img src={convertFileSrc(thumbnail.data)} alt="" className="size-full object-cover" />
      ) : (
        <div className="flex size-full items-center justify-center text-muted-foreground">
          {thumbnail.isError ? (
            <ImageOff className="size-5" />
          ) : (
            <Loader2 className="size-5 animate-spin" />
          )}
        </div>
      )}
      {asset.mediaType === "video" && (
        <span className="absolute top-1 left-1 flex items-center gap-0.5 rounded bg-black/60 px-1 text-xs text-white">
          <Play className="size-3" />
          {formatDuration(Math.round(asset.duration ?? 0))}
        </span>
      )}
      <span className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent px-1.5 pt-4 pb-1 text-xs text-white tabular-nums">
        {formatTime(asset.date)}
      </span>
      <span className="absolute top-1 right-1">
        {asset.added ? (
          <span
            className="flex size-4 items-center justify-center rounded-sm bg-primary text-primary-foreground"
            title={t("Already added")}
          >
            <Check className="size-3" />
          </span>
        ) : (
          <Checkbox
            checked={selected}
            tabIndex={-1}
            className="pointer-events-none bg-background"
          />
        )}
      </span>
    </button>
  );
}

function AccessRequest({ denied, onDecided }: { denied: boolean; onDecided: () => void }) {
  const request = useMutation({ mutationFn: api.requestPhotosAccess, onSettled: onDecided });
  return (
    <div className="grid gap-3 text-sm">
      <p className="text-muted-foreground">
        {denied
          ? t(
              "RunningMate isn't allowed to read your Photos library. Allow it in System Settings → Privacy & Security → Photos.",
            )
          : t(
              "To find the photos and videos you took during this activity, RunningMate needs to read your Photos library. Nothing is changed or uploaded.",
            )}
      </p>
      <div>
        {denied ? (
          <Button variant="outline" onClick={() => void openUrl(PRIVACY_SETTINGS)}>
            {t("Open System Settings")}
          </Button>
        ) : (
          <Button disabled={request.isPending} onClick={() => request.mutate()}>
            {t("Allow access…")}
          </Button>
        )}
      </div>
    </div>
  );
}

/**
 * "Photos from this activity": the Photos library items taken during the activity
 * (± 10 minutes), to tick and copy into the media folder.
 */
export function ActivityPhotosDialog({
  activityId,
  open,
  onOpenChange,
  onAdded,
}: {
  activityId: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded: (count: number) => void;
}) {
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const access = useQuery({ queryKey: ["photosAccess"], queryFn: api.photosAccess, enabled: open });
  const granted = access.data === "authorized" || access.data === "limited";
  const assets = useQuery({
    queryKey: ["activityPhotos", activityId],
    queryFn: () => api.activityPhotos(activityId),
    enabled: open && granted,
  });
  const available = (assets.data ?? []).filter((a) => !a.added);
  const add = useMutation({
    mutationFn: () => api.addLibraryPhotos(activityId, [...selected]),
    onSuccess: (items) => {
      onAdded(items.length);
      setSelected(new Set());
      onOpenChange(false);
    },
    onError: (error) => toast.error(t("Could not add media: {{error}}", { error: String(error) })),
    onSettled: () =>
      void queryClient.invalidateQueries({ queryKey: ["activityPhotos", activityId] }),
  });
  const toggle = (id: string) =>
    setSelected((current) => {
      const next = new Set(current);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  const allSelected = available.length > 0 && available.every((a) => selected.has(a.id));

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!add.isPending) onOpenChange(next);
        if (!next) setSelected(new Set());
      }}
    >
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("Photos from this activity")}</DialogTitle>
          <DialogDescription>
            {t("Photos and videos from your Photos library taken during the activity.")}
          </DialogDescription>
        </DialogHeader>

        {access.data && !granted && (
          <AccessRequest
            denied={access.data !== "notDetermined"}
            onDecided={() => void access.refetch()}
          />
        )}
        {granted && assets.isPending && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            {t("Searching the Photos library…")}
          </p>
        )}
        {assets.error && (
          <p className="text-sm break-words text-destructive">{String(assets.error)}</p>
        )}
        {assets.data?.length === 0 && (
          <p className="text-sm text-muted-foreground">
            {access.data === "limited"
              ? t(
                  "No photos found. RunningMate can only see the photos you shared with it; you can change this in System Settings.",
                )
              : t("No photos or videos were taken during this activity.")}
          </p>
        )}
        {assets.data && assets.data.length > 0 && (
          <div className="grid max-h-[60vh] grid-cols-4 gap-2 overflow-y-auto p-0.5">
            {assets.data.map((asset) => (
              <AssetTile
                key={asset.id}
                asset={asset}
                selected={selected.has(asset.id)}
                onToggle={() => toggle(asset.id)}
              />
            ))}
          </div>
        )}

        <DialogFooter className="sm:justify-between">
          <Button
            variant="ghost"
            disabled={available.length === 0 || add.isPending}
            onClick={() =>
              setSelected(allSelected ? new Set() : new Set(available.map((a) => a.id)))
            }
          >
            {allSelected ? t("Deselect all") : t("Select all")}
          </Button>
          <Button disabled={selected.size === 0 || add.isPending} onClick={() => add.mutate()}>
            {add.isPending && <Loader2 className="animate-spin" />}
            {add.isPending ? t("Adding…") : t("Add {{count}} file", { count: selected.size })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
