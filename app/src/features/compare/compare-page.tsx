import { useQueryClient } from "@tanstack/react-query";
import { ChartNoAxesCombined, ChevronDown, Plus, Route, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Label } from "@/components/ui/label";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
import { useSegmentEfforts } from "@/features/segments/use-segment-efforts";
import { t } from "@/i18n";
import { api } from "@/lib/api";

import { ActivityPicker } from "./activity-picker";
import { OverlayCharts, RaceChart, SplitsChart } from "./compare-charts";
import { EffectFigures, KeyFigures, Swatch, ZoneFigures } from "./compare-figures";
import {
  addActivities,
  EMPTY_SELECTION,
  MAX_COMPARE,
  onFoot,
  pickPasses,
  removeActivity,
  segmentSelection,
  speedMode,
  type CompareSelection,
} from "./compare-math";
import { useCompareData, type CompareEntry } from "./use-compare-data";

type Order = "fastest" | "latest";

/** A selected activity: opens it on click; its menu adds similar activities or segment passes. */
function Chip({
  entry,
  full,
  onOpen,
  onRemove,
  onSimilar,
  onPasses,
}: {
  entry: CompareEntry;
  full: boolean;
  onOpen: () => void;
  onRemove: () => void;
  onSimilar: () => void;
  onPasses: (segment: { id: number; title: string }, order: Order) => void;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const { data: efforts = [] } = useSegmentEfforts(entry.id, menuOpen);
  // An activity can pass a segment more than once; it's listed once.
  const segments = efforts.filter(
    (effort, i) => efforts.findIndex((e) => e.segmentId === effort.segmentId) === i,
  );
  const passes = (order: Order) => (
    <DropdownMenuSubContent className="max-w-72">
      {segments.length === 0 ? (
        <DropdownMenuItem disabled>{t("This activity passes no segment.")}</DropdownMenuItem>
      ) : (
        segments.map((effort) => (
          <DropdownMenuItem
            key={effort.segmentId}
            onSelect={() => onPasses({ id: effort.segmentId, title: effort.title }, order)}
          >
            <Route />
            <span data-private className="truncate">
              {effort.title}
            </span>
          </DropdownMenuItem>
        ))
      )}
    </DropdownMenuSubContent>
  );
  return (
    <div className="flex h-8 max-w-72 min-w-0 items-center rounded-md border bg-card text-sm">
      <button
        type="button"
        title={t("Open details")}
        className="flex h-full min-w-0 items-center gap-2 rounded-l-md pr-1 pl-2.5 hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        onClick={onOpen}
      >
        <Swatch color={entry.color} />
        <span className="shrink-0 text-muted-foreground">{entry.label}</span>
        <span data-private className="truncate">
          {entry.missing ? t("Not found") : entry.detail?.title}
        </span>
      </button>
      <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon-xs" aria-label={t("More")} title={t("More")}>
            <ChevronDown />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem onSelect={onOpen}>{t("Open details")}</DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem disabled={full || entry.missing} onSelect={onSimilar}>
            {t("Add similar activities")}
          </DropdownMenuItem>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>{t("Fastest passes of a segment")}</DropdownMenuSubTrigger>
            {passes("fastest")}
          </DropdownMenuSub>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>{t("Latest passes of a segment")}</DropdownMenuSubTrigger>
            {passes("latest")}
          </DropdownMenuSub>
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={onRemove}>{t("Remove")}</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <Button
        variant="ghost"
        size="icon-xs"
        className="mr-1"
        aria-label={t("Remove")}
        title={t("Remove")}
        onClick={onRemove}
      >
        <X />
      </Button>
    </div>
  );
}

/**
 * Up to ten activities of any sport side by side: key figures, the tracks over distance or
 * time, a virtual race, splits, training effect and heart-rate zones.
 */
export function ComparePage({
  selection,
  onSelectionChange,
  dataDir,
}: {
  selection: CompareSelection;
  onSelectionChange: (selection: CompareSelection) => void;
  dataDir?: string;
}) {
  const queryClient = useQueryClient();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [detailId, setDetailId] = useState<number | null>(null);
  const [onlySegment, setOnlySegment] = useState(true);
  // A new comparison from the segment window (opened within the detail window) closes it.
  const [current, setCurrent] = useState(selection);
  if (current !== selection) {
    setCurrent(selection);
    if (selection.segment !== current.segment) {
      setDetailId(null);
      setOnlySegment(true);
    }
  }
  const entries = useCompareData(selection);
  const shown = entries.filter((e) => !e.missing);
  const ids = selection.items.map((item) => item.id);
  const full = ids.length >= MAX_COMPARE;
  const stretched = selection.segment !== null && onlySegment;
  const views = shown.map((e) => e.view);
  const mixed = speedMode(views) === "speed" && views.some(onFoot);

  const toggle = (id: number) =>
    onSelectionChange(
      ids.includes(id) ? removeActivity(selection, id) : addActivities(selection, [id]),
    );
  const addSimilar = async (id: number) => {
    try {
      const similar = (await api.similarActivities(id, MAX_COMPARE)).filter(
        (other) => !ids.includes(other),
      );
      if (!similar.length) {
        toast.info(t("No similar activities: same sport, start within 200 m, distance ±5 %."));
        return;
      }
      const next = addActivities(selection, similar);
      onSelectionChange(next);
      toast.success(
        t("Added {{count}} similar activities", {
          count: next.items.length - selection.items.length,
        }),
      );
    } catch (error) {
      toast.error(t("Could not find similar activities: {{error}}", { error: String(error) }));
    }
  };
  const addPasses = async (id: number, segment: { id: number; title: string }, order: Order) => {
    try {
      const detail = await queryClient.fetchQuery({
        queryKey: ["routeSegment", segment.id],
        queryFn: () => api.getSegment(segment.id),
      });
      onSelectionChange(segmentSelection(segment, pickPasses(detail.leaderboard, order, id)));
    } catch (error) {
      toast.error(t("Could not load the segment: {{error}}", { error: String(error) }));
    }
  };
  const neighbour = (step: number) => {
    const next = detailId === null ? -1 : ids.indexOf(detailId) + step;
    return next >= 0 && next < ids.length && detailId !== null
      ? () => setDetailId(ids[next])
      : undefined;
  };

  return (
    // A scrolling block with a grid inside: grid rows are always as tall as their content, so no
    // card is squeezed into the window height and cut off.
    <div className="h-full overflow-y-auto">
      <div className="grid min-h-full content-start gap-4 p-4">
        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm" disabled={full} onClick={() => setPickerOpen(true)}>
            <Plus />
            {t("Add activity")}
          </Button>
          {entries.map((entry) => (
            <Chip
              key={entry.id}
              entry={entry}
              full={full}
              onOpen={() => setDetailId(entry.id)}
              onRemove={() => onSelectionChange(removeActivity(selection, entry.id))}
              onSimilar={() => void addSimilar(entry.id)}
              onPasses={(segment, order) => void addPasses(entry.id, segment, order)}
            />
          ))}
          {ids.length > 0 && (
            <>
              <span className="text-sm text-muted-foreground">
                {ids.length} / {MAX_COMPARE}
              </span>
              <Button size="sm" variant="ghost" onClick={() => onSelectionChange(EMPTY_SELECTION)}>
                {t("Clear")}
              </Button>
            </>
          )}
        </div>

        {selection.segment && (
          <div className="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
            <span className="flex min-w-0 items-center gap-1.5">
              <Route className="size-4 shrink-0 text-primary" />
              {t("Passes of the segment")}
              <span data-private className="truncate font-medium">
                {selection.segment.title}
              </span>
            </span>
            <div className="flex items-center gap-2">
              <Checkbox
                id="compare-only-segment"
                checked={onlySegment}
                onCheckedChange={(checked) => setOnlySegment(checked === true)}
              />
              <Label htmlFor="compare-only-segment">{t("Only the segment")}</Label>
            </div>
          </div>
        )}

        {ids.length === 0 ? (
          <div className="flex min-h-[50vh] flex-col items-center justify-center gap-2 text-center text-sm text-muted-foreground">
            <ChartNoAxesCombined className="size-8" />
            <p>{t("Compare up to 10 activities of any sport.")}</p>
            <p className="max-w-md">
              {t(
                "Add them with “Add activity”, or add one and use its menu to add similar activities or the passes of a segment. The segment window has a Compare button too.",
              )}
            </p>
          </div>
        ) : (
          <>
            {mixed && (
              <p className="text-xs text-muted-foreground">
                {t("With rides among them, runs and walks are shown as speed too (km/h).")}
              </p>
            )}
            {stretched && (
              <p className="text-xs text-muted-foreground">
                {t("Training effect and heart-rate zones are those of the whole activities.")}
              </p>
            )}
            <KeyFigures entries={shown} stretched={stretched} />
            <OverlayCharts entries={shown} stretched={stretched} onOpen={setDetailId} />
            <RaceChart entries={shown} stretched={stretched} onOpen={setDetailId} />
            <SplitsChart entries={shown} stretched={stretched} onOpen={setDetailId} />
            <EffectFigures entries={shown} />
            <ZoneFigures entries={shown} />
          </>
        )}

        <ActivityPicker
          open={pickerOpen}
          onOpenChange={setPickerOpen}
          selected={ids}
          onToggle={toggle}
        />
        <ActivityDetailDialog
          activityId={detailId}
          onClose={() => setDetailId(null)}
          onPrevious={neighbour(-1)}
          onNext={neighbour(1)}
          dataDir={dataDir}
        />
      </div>
    </div>
  );
}
