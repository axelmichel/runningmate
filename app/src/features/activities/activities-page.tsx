import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";

import type { ActivityFilter } from "@/bindings/ActivityFilter";
import type { ViewMode } from "@/bindings/ViewMode";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
import { invalidateGoals } from "@/features/goals/goal-math";
import { invalidateTraining } from "@/features/training/training-math";
import { invalidateSegments } from "@/features/segments/segment-math";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

import { ActivityHeatmap } from "./activity-heatmap";
import { ActivityPanel } from "./activity-panel";
import { ActivityTable, type Sort } from "./activity-table";
import { heatmapEnd } from "./heatmap";
import { InfoCards } from "./info-cards";
import { SearchBar } from "./search-bar";

const PAGE_SIZE = 25;
const DEFAULT_SORT: Sort = { field: "date", direction: "desc" };

type ActivitiesPageProps = {
  view: ViewMode;
  searchOpen: boolean;
  onSearchOpenChange: (open: boolean) => void;
  /** Whether the right-hand details panel is shown. */
  detailsOpen: boolean;
  dataDir?: string;
};

export function ActivitiesPage({
  view,
  searchOpen,
  onSearchOpenChange,
  detailsOpen,
  dataDir,
}: ActivitiesPageProps) {
  const queryClient = useQueryClient();
  const [filter, setFilter] = useState<ActivityFilter>({});
  const [sort, setSort] = useState<Sort>(DEFAULT_SORT);
  const [pageIndex, setPageIndex] = useState(0);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [detailId, setDetailId] = useState<number | null>(null);

  // Switching view keeps the filter but starts over on page one, as the Python app did.
  const [shownView, setShownView] = useState(view);
  if (shownView !== view) {
    setShownView(view);
    setPageIndex(0);
    setActiveId(null);
    setSort(DEFAULT_SORT);
  }

  const changeFilter = useCallback((next: ActivityFilter) => {
    setFilter(next);
    setPageIndex(0);
  }, []);

  const page = useQuery({
    queryKey: ["activities", view, filter, sort, pageIndex],
    queryFn: () =>
      api.listActivities({
        viewMode: view,
        offset: pageIndex * PAGE_SIZE,
        limit: PAGE_SIZE,
        sortField: sort.field,
        sortDirection: sort.direction,
        filter,
      }),
    placeholderData: keepPreviousData,
  });
  const summary = useQuery({
    queryKey: ["activities", "summary", view, filter],
    queryFn: () => api.activitySummary(view, filter),
  });
  const heatmap = useQuery({
    queryKey: ["activities", "heatmap", view, filter],
    queryFn: () => api.activityHeatmap(view, filter),
  });
  // Without a selection the panel shows the latest activity of the view.
  const latest = useQuery({
    queryKey: ["activities", "latest", view],
    queryFn: () =>
      api.listActivities({
        viewMode: view,
        offset: 0,
        limit: 1,
        sortField: "date",
        sortDirection: "desc",
        filter: {},
      }),
  });

  const remove = useMutation({
    mutationFn: (id: number) => api.deleteActivity(id),
    onSuccess: (_, id) => {
      toast.success(t("Activity deleted"));
      if (activeId === id) setActiveId(null);
      queryClient.invalidateQueries({ queryKey: ["activities"] });
      invalidateSegments(queryClient);
      invalidateGoals(queryClient);
      invalidateTraining(queryClient);
    },
    onError: (error) =>
      toast.error(t("Could not delete the activity: {{error}}", { error: String(error) })),
  });

  const reimport = useMutation({
    mutationFn: (id: number) => api.reimportActivity(id),
    onSuccess: (_, id) => {
      toast.success(t("Activity recomputed from its TCX file"));
      queryClient.invalidateQueries({ queryKey: ["activities"] });
      queryClient.invalidateQueries({ queryKey: ["activity", id] });
      queryClient.invalidateQueries({ queryKey: ["bestPerformances", id] });
      invalidateSegments(queryClient);
      invalidateGoals(queryClient);
      invalidateTraining(queryClient);
    },
    onError: (error) => toast.error(t("Could not re-import: {{error}}", { error: String(error) })),
  });

  const uploadGarmin = useMutation({
    mutationFn: (id: number) => api.garminUpload([id]),
    onSuccess: (summary, id) => {
      if (summary.imported > 0) toast.success(t("Uploaded to Garmin Connect"));
      else if (summary.duplicates > 0) toast.info(t("Garmin Connect already has this activity"));
      else toast.error(t("The upload to Garmin Connect failed"));
      queryClient.invalidateQueries({ queryKey: ["links", id] });
    },
    onError: (error) =>
      toast.error(
        String(error).includes("garmin-sign-in-required")
          ? t("Sign in to Garmin Connect under Connectors first.")
          : t("The upload to Garmin Connect failed"),
      ),
  });

  // Previous/next in the detail window follow the table order across pages: within the page
  // the neighbour is at hand, at a page edge it's fetched and the table turns the page too.
  const rows = page.data?.rows ?? [];
  const total = page.data?.total ?? 0;
  const detailIndex = detailId === null ? -1 : rows.findIndex((row) => row.id === detailId);
  const detailPosition = detailIndex < 0 ? -1 : pageIndex * PAGE_SIZE + detailIndex;
  const showActivity = (id: number) => {
    setDetailId(id);
    setActiveId(id);
  };
  const step = async (delta: -1 | 1) => {
    const neighbour = rows[detailIndex + delta];
    if (neighbour) {
      showActivity(neighbour.id);
      return;
    }
    const position = detailPosition + delta;
    try {
      const found = await queryClient.fetchQuery({
        queryKey: ["activities", "at", view, filter, sort, position],
        queryFn: () =>
          api.listActivities({
            viewMode: view,
            offset: position,
            limit: 1,
            sortField: sort.field,
            sortDirection: sort.direction,
            filter,
          }),
      });
      const next = found.rows[0];
      if (!next) return;
      setPageIndex(Math.floor(position / PAGE_SIZE));
      showActivity(next.id);
    } catch (error) {
      toast.error(String(error));
    }
  };
  const canStep = (delta: -1 | 1) =>
    detailPosition >= 0 && detailPosition + delta >= 0 && detailPosition + delta < total;

  const heatmapEndDate = useMemo(() => heatmapEnd(filter), [filter]);
  const panelId = activeId ?? latest.data?.rows[0]?.id;

  return (
    <div className="flex h-full min-h-0 p-4">
      <div className="flex min-w-0 flex-1 flex-col gap-4">
        {searchOpen && (
          <SearchBar
            filter={filter}
            onFilterChange={changeFilter}
            onClose={() => onSearchOpenChange(false)}
          />
        )}
        <InfoCards summary={summary.data} />
        <Card size="sm">
          <CardContent>
            <ActivityHeatmap days={heatmap.data ?? []} end={heatmapEndDate} />
          </CardContent>
        </Card>
        <ActivityTable
          view={view}
          page={page.data}
          pageIndex={pageIndex}
          pageSize={PAGE_SIZE}
          onPageChange={setPageIndex}
          sort={sort}
          onSortChange={(next) => {
            setSort(next);
            setPageIndex(0);
          }}
          activeId={panelId ?? null}
          onActivate={setActiveId}
          onDelete={(row) => remove.mutate(row.id)}
          onReimport={(row) => reimport.mutate(row.id)}
          onOpenDetails={(row) => showActivity(row.id)}
          onUploadGarmin={(row) => uploadGarmin.mutate(row.id)}
        />
      </div>
      {/* Slides in and out like the left sidebar; the inner width stays fixed while animating. */}
      <aside
        aria-hidden={!detailsOpen}
        inert={!detailsOpen}
        className={cn(
          "shrink-0 overflow-hidden transition-[width] duration-200 ease-linear",
          detailsOpen ? "w-84" : "w-0",
        )}
      >
        <ScrollArea className="h-full w-84 pl-4">
          <ActivityPanel activityId={panelId} dataDir={dataDir} onOpenDetails={showActivity} />
        </ScrollArea>
      </aside>
      <ActivityDetailDialog
        activityId={detailId}
        onClose={() => setDetailId(null)}
        onPrevious={canStep(-1) ? () => void step(-1) : undefined}
        onNext={canStep(1) ? () => void step(1) : undefined}
        dataDir={dataDir}
      />
    </div>
  );
}
