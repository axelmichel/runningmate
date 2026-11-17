import { convertFileSrc } from "@tauri-apps/api/core";
import { ChevronLeft, ChevronRight, ExternalLink, Pause, Play } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { Media } from "@/bindings/Media";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { t } from "@/i18n";
import { cn } from "@/lib/utils";

/** Seconds a photo shows in the slideshow; videos play to their end. */
const SLIDE_SECONDS = 4;

export type LightboxItem = {
  media: Media;
  /** Shown below the photo, e.g. the activity and its date. */
  caption?: string;
  activityId?: number;
};

/**
 * Photos and videos in full, one at a time: previous/next buttons and arrow keys, a strip of
 * thumbnails to jump with, and a slideshow. `index` is the item shown, `null` closes it.
 */
export function MediaLightbox({
  items,
  index,
  onIndexChange,
  onOpenActivity,
}: {
  items: LightboxItem[];
  index: number | null;
  onIndexChange: (index: number | null) => void;
  /** Shows a button that opens the item's activity. */
  onOpenActivity?: (activityId: number) => void;
}) {
  const [playing, setPlaying] = useState(false);
  const strip = useRef<HTMLDivElement>(null);
  const open = index !== null && items.length > 0;
  const current = open ? items[Math.min(index, items.length - 1)] : null;
  const count = items.length;
  const go = (step: number) => {
    if (index === null || count === 0) return;
    onIndexChange((index + step + count) % count);
  };

  // A photo moves on after a few seconds; a video when it ends (see onEnded).
  useEffect(() => {
    if (!playing || !current || current.media.mediaType === "video") return;
    const timer = setTimeout(() => go(1), SLIDE_SECONDS * 1000);
    return () => clearTimeout(timer);
  });

  // Keeps the current thumbnail in view.
  useEffect(() => {
    strip.current
      ?.querySelector<HTMLElement>(`[data-index="${index}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "center", behavior: "smooth" });
  }, [index]);

  return (
    <Dialog
      open={open}
      onOpenChange={(value) => {
        if (value) return;
        setPlaying(false);
        onIndexChange(null);
      }}
    >
      <DialogContent
        className="flex h-[90vh] max-w-6xl flex-col gap-2 p-2 sm:max-w-6xl"
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft") go(-1);
          else if (event.key === "ArrowRight") go(1);
          else if (event.key === " " && current?.media.mediaType !== "video") {
            event.preventDefault();
            setPlaying((value) => !value);
          } else return;
          event.stopPropagation();
        }}
      >
        <DialogTitle className="sr-only">{t("Media")}</DialogTitle>
        <DialogDescription className="sr-only">{current?.caption}</DialogDescription>
        <div className="relative flex min-h-0 flex-1 items-center justify-center">
          {current?.media.mediaType === "video" ? (
            <video
              key={current.media.id}
              src={convertFileSrc(current.media.filePath)}
              controls
              autoPlay
              onEnded={() => playing && go(1)}
              className="max-h-full max-w-full"
            />
          ) : (
            current && (
              <img
                key={current.media.id}
                src={convertFileSrc(current.media.filePath)}
                alt={current.caption ?? ""}
                className="max-h-full max-w-full object-contain"
              />
            )
          )}
          {count > 1 && (
            <>
              <Button
                variant="secondary"
                size="icon"
                aria-label={t("Previous")}
                className="absolute top-1/2 left-2 -translate-y-1/2 rounded-full opacity-80"
                onClick={() => go(-1)}
              >
                <ChevronLeft />
              </Button>
              <Button
                variant="secondary"
                size="icon"
                aria-label={t("Next")}
                className="absolute top-1/2 right-2 -translate-y-1/2 rounded-full opacity-80"
                onClick={() => go(1)}
              >
                <ChevronRight />
              </Button>
            </>
          )}
        </div>
        <div className="flex items-center gap-2 px-1 text-sm">
          <span className="min-w-0 flex-1 truncate text-muted-foreground">
            <span data-private>{current?.caption}</span>
            {count > 1 && index !== null && (
              <span className="ml-2 tabular-nums">
                {index + 1} / {count}
              </span>
            )}
          </span>
          {count > 1 && (
            <Button size="sm" variant="ghost" onClick={() => setPlaying((value) => !value)}>
              {playing ? <Pause /> : <Play />}
              {playing ? t("Pause") : t("Slideshow")}
            </Button>
          )}
          {onOpenActivity && current?.activityId != null && (
            <Button
              size="sm"
              variant="outline"
              onClick={() => {
                setPlaying(false);
                onOpenActivity(current.activityId!);
              }}
            >
              <ExternalLink />
              {t("Open activity")}
            </Button>
          )}
        </div>
        {count > 1 && (
          <div ref={strip} className="flex shrink-0 gap-1.5 overflow-x-auto px-1 pb-1">
            {items.map((item, i) => (
              <button
                key={item.media.id}
                type="button"
                data-index={i}
                aria-label={item.caption ?? t("Open")}
                className={cn(
                  "relative size-14 shrink-0 overflow-hidden rounded border-2 opacity-60 transition-opacity hover:opacity-100",
                  i === index ? "border-primary opacity-100" : "border-transparent",
                )}
                onClick={() => onIndexChange(i)}
              >
                {item.media.mediaType === "video" ? (
                  <>
                    <video
                      src={convertFileSrc(item.media.filePath)}
                      preload="metadata"
                      muted
                      className="size-full object-cover"
                    />
                    <Play className="absolute inset-0 m-auto size-4 text-white drop-shadow" />
                  </>
                ) : (
                  <img
                    src={convertFileSrc(item.media.filePath)}
                    alt=""
                    loading="lazy"
                    className="size-full object-cover"
                  />
                )}
              </button>
            ))}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
