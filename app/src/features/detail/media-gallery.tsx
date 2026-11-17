import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as pickFiles } from "@tauri-apps/plugin-dialog";
import { CalendarClock, FileWarning, FolderOpen, Images, Link2, Play, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import type { Media } from "@/bindings/Media";
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
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { MediaLightbox } from "@/components/media-lightbox";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { isMac } from "@/lib/platform";
import { cn } from "@/lib/utils";

import { ActivityPhotosDialog } from "./activity-photos-dialog";
import { MEDIA_EXTENSIONS, isMediaFile, mediaMode, setMediaMode } from "./media-mode";

function Thumbnail({
  media,
  onOpen,
  onDelete,
}: {
  media: Media;
  onOpen: () => void;
  onDelete: () => void;
}) {
  const src = convertFileSrc(media.filePath);
  return (
    <div className="group relative aspect-square overflow-hidden rounded-md border bg-muted">
      {media.missing ? (
        <div
          className="flex size-full flex-col items-center justify-center gap-1 p-2 text-center text-xs text-muted-foreground"
          title={media.filePath}
        >
          <FileWarning className="size-5" />
          {t("File missing")}
        </div>
      ) : (
        <button type="button" className="size-full" onClick={onOpen} aria-label={t("Open")}>
          {media.mediaType === "video" ? (
            <>
              {/* The first frame serves as the thumbnail (Python used OpenCV for this). */}
              <video src={src} preload="metadata" muted className="size-full object-cover" />
              <Play className="absolute inset-0 m-auto size-8 text-white drop-shadow" />
            </>
          ) : (
            <img src={src} alt="" loading="lazy" className="size-full object-cover" />
          )}
        </button>
      )}
      {media.linked && (
        <span
          className="absolute bottom-1 left-1 rounded bg-black/60 p-0.5 text-white"
          title={t("Linked to {{path}}", { path: media.filePath })}
        >
          <Link2 className="size-3.5" />
        </span>
      )}
      <Button
        variant="secondary"
        size="icon-sm"
        aria-label={t("Remove")}
        className="absolute top-1 right-1 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        onClick={onDelete}
      >
        <X />
      </Button>
    </div>
  );
}

/** Asks whether files from disk are copied or linked; can remember the answer. */
function AddModeDialog({
  files,
  onChoose,
  onCancel,
}: {
  files: string[];
  onChoose: (link: boolean, remember: boolean) => void;
  onCancel: () => void;
}) {
  const [remember, setRemember] = useState(false);
  return (
    <Dialog open={files.length > 0} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("Add {{count}} file", { count: files.length })}</DialogTitle>
          <DialogDescription>
            {t(
              "Copying keeps the files safe in RunningMate's media folder. Linking saves space but breaks if the originals are moved or deleted.",
            )}
          </DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-2">
          <Checkbox
            id="remember-media-mode"
            checked={remember}
            onCheckedChange={(checked) => setRemember(checked === true)}
          />
          <Label htmlFor="remember-media-mode" className="font-normal">
            {t("Remember my choice (can be changed in the settings)")}
          </Label>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onChoose(true, remember)}>
            <Link2 />
            {t("Link to originals")}
          </Button>
          <Button onClick={() => onChoose(false, remember)}>{t("Copy into RunningMate")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/**
 * Photos and videos of an activity: from the Photos library (those taken during the activity,
 * or any via the macOS picker), from disk, or dropped onto the gallery; with a viewer and delete.
 */
export function MediaGallery({ activityId }: { activityId: number }) {
  const queryClient = useQueryClient();
  const [viewing, setViewing] = useState<Media | null>(null);
  const [deleting, setDeleting] = useState<Media | null>(null);
  const [pending, setPending] = useState<string[]>([]);
  const [dropping, setDropping] = useState(false);
  const [libraryOpen, setLibraryOpen] = useState(false);
  const area = useRef<HTMLDivElement>(null);
  const { data: media = [] } = useQuery({
    queryKey: ["media", activityId],
    queryFn: () => api.listMedia(activityId),
  });
  const viewable = media.filter((m) => !m.missing);
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: ["media", activityId] });
    void queryClient.invalidateQueries({ queryKey: ["gallery"] });
    void queryClient.invalidateQueries({ queryKey: ["recentPhotos"] });
    // Media makes an activity count as reviewed.
    void queryClient.invalidateQueries({ queryKey: ["activity", activityId] });
  };
  const added = (count: number) => {
    if (count > 0) toast.success(t("{{count}} file added", { count }));
  };
  const failed = (error: unknown) =>
    toast.error(t("Could not add media: {{error}}", { error: String(error) }));

  const addFiles = useMutation({
    mutationFn: async ({ files, link }: { files: string[]; link: boolean }) => {
      for (const file of files) await api.addMedia(activityId, file, link);
      return files.length;
    },
    onSuccess: added,
    onError: failed,
    onSettled: refresh,
  });
  const fromPhotos = useMutation({
    mutationFn: () => api.addMediaFromPhotos(activityId),
    onSuccess: (items) => added(items.length),
    onError: failed,
    onSettled: refresh,
  });
  const remove = useMutation({
    mutationFn: (id: number) => api.deleteMedia(id),
    onError: (error) =>
      toast.error(t("Could not remove media: {{error}}", { error: String(error) })),
    onSettled: refresh,
  });

  /** Adds files from disk, asking copy or link unless a choice was remembered. */
  const offer = (files: string[]) => {
    const usable = files.filter(isMediaFile);
    if (usable.length < files.length) {
      toast.warning(t("Only photos and videos can be added."));
    }
    if (usable.length === 0) return;
    const mode = mediaMode();
    if (mode === "ask") setPending(usable);
    else addFiles.mutate({ files: usable, link: mode === "link" });
  };

  const pickFromDisk = async () => {
    const files = await pickFiles({
      multiple: true,
      filters: [{ name: t("Photos and videos"), extensions: MEDIA_EXTENSIONS }],
    });
    if (files?.length) offer(files);
  };

  // Files dragged from Finder (or Photos) onto the gallery. Tauri reports drops for the whole
  // window with physical coordinates, so check whether they land on this gallery.
  const offerRef = useRef(offer);
  useEffect(() => {
    offerRef.current = offer;
  });
  useEffect(() => {
    const inside = (x: number, y: number) => {
      const box = area.current?.getBoundingClientRect();
      const scale = window.devicePixelRatio || 1;
      const [cx, cy] = [x / scale, y / scale];
      return Boolean(box && cx >= box.left && cx <= box.right && cy >= box.top && cy <= box.bottom);
    };
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "leave") setDropping(false);
      else if (payload.type === "enter" || payload.type === "over") {
        setDropping(inside(payload.position.x, payload.position.y));
      } else if (payload.type === "drop") {
        setDropping(false);
        if (inside(payload.position.x, payload.position.y)) offerRef.current(payload.paths);
      }
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const busy = addFiles.isPending || fromPhotos.isPending;

  return (
    <div
      ref={area}
      className={cn(
        "grid gap-2 rounded-lg p-1 transition-colors",
        dropping && "bg-primary/10 ring-2 ring-primary ring-inset",
      )}
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="text-sm font-medium">{t("Photos and videos")}</span>
        <div className="flex gap-2">
          {isMac && (
            <>
              <Button
                variant="outline"
                size="sm"
                disabled={busy}
                onClick={() => setLibraryOpen(true)}
              >
                <CalendarClock />
                {t("From this activity…")}
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={busy}
                onClick={() => fromPhotos.mutate()}
              >
                <Images />
                {t("From Photos…")}
              </Button>
            </>
          )}
          <Button variant="outline" size="sm" disabled={busy} onClick={() => void pickFromDisk()}>
            <FolderOpen />
            {t("From files…")}
          </Button>
        </div>
      </div>
      {media.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          {t("No photos or videos yet. Add some, or drop them here.")}
        </p>
      ) : (
        <div className="grid grid-cols-3 gap-2">
          {media.map((m) => (
            <Thumbnail
              key={m.id}
              media={m}
              onOpen={() => setViewing(m)}
              onDelete={() => setDeleting(m)}
            />
          ))}
        </div>
      )}

      {isMac && (
        <ActivityPhotosDialog
          activityId={activityId}
          open={libraryOpen}
          onOpenChange={setLibraryOpen}
          onAdded={(count) => {
            added(count);
            refresh();
          }}
        />
      )}

      <AddModeDialog
        files={pending}
        onCancel={() => setPending([])}
        onChoose={(link, remember) => {
          if (remember) setMediaMode(link ? "link" : "copy");
          addFiles.mutate({ files: pending, link });
          setPending([]);
        }}
      />

      <MediaLightbox
        items={viewable.map((m) => ({ media: m }))}
        index={
          viewing === null
            ? null
            : Math.max(
                0,
                viewable.findIndex((m) => m.id === viewing.id),
              )
        }
        onIndexChange={(index) => setViewing(index === null ? null : viewable[index])}
      />

      <AlertDialog open={deleting !== null} onOpenChange={(open) => !open && setDeleting(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Remove this file?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {deleting?.linked
                ? t("Only the link is removed; the original file is kept.")
                : t("The file is deleted from the media folder.")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => deleting && remove.mutate(deleting.id)}
            >
              {t("Remove")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
