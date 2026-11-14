import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open as pickFiles } from "@tauri-apps/plugin-dialog";
import { ImagePlus, Play, X } from "lucide-react";
import { useState } from "react";
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
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { api } from "@/lib/api";

const EXTENSIONS = ["png", "jpg", "jpeg", "heic", "mp4", "mov", "avi"];

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
      <button type="button" className="size-full" onClick={onOpen} aria-label="Open">
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
      <Button
        variant="secondary"
        size="icon-sm"
        aria-label="Remove"
        className="absolute top-1 right-1 opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
        onClick={onDelete}
      >
        <X />
      </Button>
    </div>
  );
}

/** Photos and videos of an activity, with a viewer, upload and delete. */
export function MediaGallery({ activityId }: { activityId: number }) {
  const queryClient = useQueryClient();
  const [viewing, setViewing] = useState<Media | null>(null);
  const [deleting, setDeleting] = useState<Media | null>(null);
  const { data: media = [] } = useQuery({
    queryKey: ["media", activityId],
    queryFn: () => api.listMedia(activityId),
  });
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["media", activityId] });

  const add = useMutation({
    mutationFn: async () => {
      const files = await pickFiles({
        multiple: true,
        filters: [{ name: "Photos and videos", extensions: EXTENSIONS }],
      });
      for (const file of files ?? []) await api.addMedia(activityId, file);
      return files?.length ?? 0;
    },
    onSuccess: (count) => count && toast.success(`${count} file${count === 1 ? "" : "s"} added`),
    onError: (error) => toast.error(`Could not add media: ${error}`),
    onSettled: refresh,
  });
  const remove = useMutation({
    mutationFn: (id: number) => api.deleteMedia(id),
    onError: (error) => toast.error(`Could not remove media: ${error}`),
    onSettled: refresh,
  });

  return (
    <div className="grid gap-2">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">Photos and videos</span>
        <Button variant="outline" size="sm" disabled={add.isPending} onClick={() => add.mutate()}>
          <ImagePlus />
          Add
        </Button>
      </div>
      {media.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No photos or videos yet. Added files are copied into the media folder.
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

      <Dialog open={viewing !== null} onOpenChange={(open) => !open && setViewing(null)}>
        <DialogContent className="max-w-4xl p-2 sm:max-w-4xl">
          <DialogTitle className="sr-only">Media</DialogTitle>
          {viewing?.mediaType === "video" ? (
            <video
              src={convertFileSrc(viewing.filePath)}
              controls
              autoPlay
              className="max-h-[80vh] w-full"
            />
          ) : (
            viewing && (
              <img
                src={convertFileSrc(viewing.filePath)}
                alt=""
                className="max-h-[80vh] w-full object-contain"
              />
            )
          )}
        </DialogContent>
      </Dialog>

      <AlertDialog open={deleting !== null} onOpenChange={(open) => !open && setDeleting(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove this file?</AlertDialogTitle>
            <AlertDialogDescription>
              The file is deleted from the media folder.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => deleting && remove.mutate(deleting.id)}
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
