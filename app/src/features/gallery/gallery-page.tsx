import { useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Images, LayoutGrid, Link2, ListTree, Play } from "lucide-react";
import { useMemo, useState } from "react";

import type { GalleryItem } from "@/bindings/GalleryItem";
import { MediaLightbox, type LightboxItem } from "@/components/media-lightbox";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate } from "@/lib/format";

type Layout = "grid" | "activities";

const LAYOUT_STORAGE_KEY = "runningmate-gallery-layout";

const LAYOUTS: { layout: Layout; label: string; icon: typeof LayoutGrid }[] = [
  { layout: "grid", label: tk("Grid"), icon: LayoutGrid },
  { layout: "activities", label: tk("By activity"), icon: ListTree },
];

function caption(item: GalleryItem): string {
  return [item.activityTitle, formatDate(item.activityDate)].filter(Boolean).join(" · ");
}

function Tile({ item, onOpen }: { item: GalleryItem; onOpen: () => void }) {
  const { media } = item;
  const src = convertFileSrc(media.filePath);
  return (
    <button
      type="button"
      title={caption(item)}
      className="relative aspect-square overflow-hidden rounded-md bg-muted transition-opacity hover:opacity-85 focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
      onClick={onOpen}
    >
      {media.mediaType === "video" ? (
        <>
          {/* The first frame serves as the thumbnail. */}
          <video src={src} preload="metadata" muted className="size-full object-cover" />
          <Play className="absolute inset-0 m-auto size-8 text-white drop-shadow" />
        </>
      ) : (
        <img src={src} alt="" loading="lazy" className="size-full object-cover" />
      )}
      {media.linked && (
        <span className="absolute bottom-1 left-1 rounded bg-black/60 p-0.5 text-white">
          <Link2 className="size-3.5" />
        </span>
      )}
    </button>
  );
}

const GRID = "grid grid-cols-[repeat(auto-fill,minmax(9rem,1fr))] gap-2";

/** Every photo and video of all activities, as one grid or grouped by activity. */
export function GalleryPage({ dataDir }: { dataDir?: string }) {
  const [layout, setLayout] = useState<Layout>(() =>
    localStorage.getItem(LAYOUT_STORAGE_KEY) === "activities" ? "activities" : "grid",
  );
  const [view, setView] = useState<ViewMode>("All");
  const [viewing, setViewing] = useState<number | null>(null);
  const [detailId, setDetailId] = useState<number | null>(null);
  const { data = [], isPending } = useQuery({ queryKey: ["gallery"], queryFn: api.galleryMedia });

  const items = useMemo(
    () => data.filter((item) => view === "All" || item.viewMode === view),
    [data, view],
  );
  // Items of one activity follow each other, so grouping keeps the order of the grid.
  const groups = useMemo(() => {
    const out: { activityId: number; first: number; items: GalleryItem[] }[] = [];
    items.forEach((item, i) => {
      const last = out.at(-1);
      if (last?.activityId === item.activityId) last.items.push(item);
      else out.push({ activityId: item.activityId, first: i, items: [item] });
    });
    return out;
  }, [items]);
  const slides = useMemo<LightboxItem[]>(
    () =>
      items.map((item) => ({
        media: item.media,
        caption: caption(item),
        activityId: item.activityId,
      })),
    [items],
  );
  const activityIds = groups.map((g) => g.activityId);
  const neighbour = (step: number) => {
    const next = detailId === null ? -1 : activityIds.indexOf(detailId) + step;
    return next >= 0 && next < activityIds.length && detailId !== null
      ? () => setDetailId(activityIds[next])
      : undefined;
  };
  const chooseLayout = (next: Layout) => {
    localStorage.setItem(LAYOUT_STORAGE_KEY, next);
    setLayout(next);
  };

  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto p-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap gap-2">
          {VIEW_MODES.map(({ mode, label, icon: Icon }) => (
            <Button
              key={mode}
              size="sm"
              variant={view === mode ? "default" : "outline"}
              onClick={() => setView(mode)}
            >
              <Icon />
              {t(label)}
            </Button>
          ))}
        </div>
        <div className="flex items-center gap-3">
          <span className="text-sm text-muted-foreground">
            {t("{{count}} photos and videos", { count: items.length })}
          </span>
          <div className="flex rounded-md border p-0.5">
            {LAYOUTS.map(({ layout: value, label, icon: Icon }) => (
              <Button
                key={value}
                size="sm"
                variant={layout === value ? "secondary" : "ghost"}
                aria-pressed={layout === value}
                onClick={() => chooseLayout(value)}
              >
                <Icon />
                {t(label)}
              </Button>
            ))}
          </div>
        </div>
      </div>

      {isPending ? (
        <div className={GRID}>
          {Array.from({ length: 12 }, (_, i) => (
            <Skeleton key={i} className="aspect-square" />
          ))}
        </div>
      ) : items.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 text-sm text-muted-foreground">
          <Images className="size-8" />
          {t("No photos or videos yet. Add them to an activity on its edit page.")}
        </div>
      ) : layout === "grid" ? (
        <div className={GRID}>
          {items.map((item, i) => (
            <Tile key={item.media.id} item={item} onOpen={() => setViewing(i)} />
          ))}
        </div>
      ) : (
        <div className="grid gap-6">
          {groups.map((group) => {
            const head = group.items[0];
            const Icon = VIEW_MODES.find((v) => v.mode === head.viewMode)?.icon ?? Images;
            return (
              <section key={group.activityId} className="grid gap-2">
                <button
                  type="button"
                  title={t("Open details")}
                  className="flex w-fit items-baseline gap-2 rounded-md text-left hover:underline focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
                  onClick={() => setDetailId(group.activityId)}
                >
                  <Icon className="size-4 shrink-0 self-center text-primary" />
                  <span data-private className="font-semibold">
                    {head.activityTitle}
                  </span>
                  <span className="text-sm text-muted-foreground">
                    {formatDate(head.activityDate)}
                  </span>
                </button>
                <div className={GRID}>
                  {group.items.map((item, i) => (
                    <Tile
                      key={item.media.id}
                      item={item}
                      onOpen={() => setViewing(group.first + i)}
                    />
                  ))}
                </div>
              </section>
            );
          })}
        </div>
      )}

      <MediaLightbox
        items={slides}
        index={viewing}
        onIndexChange={setViewing}
        onOpenActivity={(id) => {
          setViewing(null);
          setDetailId(id);
        }}
      />
      <ActivityDetailDialog
        activityId={detailId}
        onClose={() => setDetailId(null)}
        onPrevious={neighbour(-1)}
        onNext={neighbour(1)}
        dataDir={dataDir}
      />
    </div>
  );
}
