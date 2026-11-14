import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";

import type { ActivityFilter } from "@/bindings/ActivityFilter";
import type { ViewMode } from "@/bindings/ViewMode";
import { Card, CardContent } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ActivityDetailDialog } from "@/features/detail/activity-detail-dialog";
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
  const [selection, setSelection] = useState<Set<number>>(new Set());
  const [detailId, setDetailId] = useState<number | null>(null);

  // Switching view keeps the filter but starts over on page one, as the Python app did.
  const [shownView, setShownView] = useState(view);
  if (shownView !== view) {
    setShownView(view);
    setPageIndex(0);
    setActiveId(null);
    setSelection(new Set());
    setSort(DEFAULT_SORT);
  }

  const changeFilter = useCallback((next: ActivityFilter) => {
    setFilter(next);
    setPageIndex(0);
    setSelection(new Set());
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
      toast.success("Activity deleted");
      if (activeId === id) setActiveId(null);
      setSelection((current) => {
        const next = new Set(current);
        next.delete(id);
        return next;
      });
      queryClient.invalidateQueries({ queryKey: ["activities"] });
    },
    onError: (error) => toast.error(`Could not delete the activity: ${error}`),
  });

  const reimport = useMutation({
    mutationFn: (id: number) => api.reimportActivity(id),
    onSuccess: (_, id) => {
      toast.success("Activity recomputed from its TCX file");
      queryClient.invalidateQueries({ queryKey: ["activities"] });
      queryClient.invalidateQueries({ queryKey: ["activity", id] });
      queryClient.invalidateQueries({ queryKey: ["bestPerformances", id] });
    },
    onError: (error) => toast.error(`Could not re-import: ${error}`),
  });

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
          selection={selection}
          onSelectionChange={setSelection}
          onDelete={(row) => remove.mutate(row.id)}
          onReimport={(row) => reimport.mutate(row.id)}
          onOpenDetails={(row) => setDetailId(row.id)}
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
          <ActivityPanel activityId={panelId} dataDir={dataDir} onOpenDetails={setDetailId} />
        </ScrollArea>
      </aside>
      <ActivityDetailDialog
        activityId={detailId}
        onClose={() => setDetailId(null)}
        dataDir={dataDir}
      />
    </div>
  );
}
