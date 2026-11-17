import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { Check, Search } from "lucide-react";
import { useDeferredValue, useState } from "react";

import type { ActivityQuery } from "@/bindings/ActivityQuery";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { withSearch } from "@/features/activities/search-query";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatDuration, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { MAX_COMPARE } from "./compare-math";

/** Results shown at once; the search narrows them down. */
const LIMIT = 50;

/**
 * Searchable list of activities (the same search as the activity list: words, place names and
 * distance shortcuts like "10km" or ">21"); clicking a row adds or removes it.
 */
export function ActivityPicker({
  open,
  onOpenChange,
  selected,
  onToggle,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  selected: number[];
  onToggle: (id: number) => void;
}) {
  const [text, setText] = useState("");
  const [view, setView] = useState<ViewMode>("All");
  const search = useDeferredValue(text.trim());
  const query: ActivityQuery = {
    viewMode: view,
    offset: 0,
    limit: LIMIT,
    sortField: "date",
    sortDirection: "desc",
    filter: withSearch(search ? { searchText: search } : {}),
  };
  const { data, isPending } = useQuery({
    queryKey: ["activities", "comparePicker", query],
    queryFn: () => api.listActivities(query),
    placeholderData: keepPreviousData,
    enabled: open,
  });
  const full = selected.length >= MAX_COMPARE;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex h-[80vh] flex-col gap-3 sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("Add activities")}</DialogTitle>
          <DialogDescription>
            {t("Search by title, comment or place, or by distance: 10km, >21, 5-10km.")}
          </DialogDescription>
        </DialogHeader>
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            autoFocus
            value={text}
            placeholder={t("Search")}
            aria-label={t("Search")}
            className="pl-8"
            onChange={(event) => setText(event.target.value)}
          />
        </div>
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
        <div className="scrollbar-visible -mx-1 min-h-0 flex-1 overflow-y-auto px-1">
          {isPending ? (
            <div className="grid gap-1">
              {Array.from({ length: 8 }, (_, i) => (
                <Skeleton key={i} className="h-11" />
              ))}
            </div>
          ) : !data?.rows.length ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              {t("No activities found.")}
            </p>
          ) : (
            <ul className="grid gap-0.5">
              {data.rows.map((row) => {
                const picked = selected.includes(row.id);
                const Icon = VIEW_MODES.find((v) => v.mode === row.viewMode)?.icon;
                return (
                  <li key={row.id}>
                    <button
                      type="button"
                      aria-pressed={picked}
                      disabled={!picked && full}
                      className={cn(
                        "flex w-full items-center gap-3 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none disabled:opacity-50",
                        picked && "bg-muted",
                      )}
                      onClick={() => onToggle(row.id)}
                    >
                      <span className="flex size-4 shrink-0 items-center justify-center">
                        {picked ? (
                          <Check className="size-4 text-primary" />
                        ) : (
                          Icon && <Icon className="size-4 text-muted-foreground" />
                        )}
                      </span>
                      <span className="min-w-0 flex-1">
                        <span data-private className="block truncate font-medium">
                          {row.title || t(row.displayType ?? "")}
                        </span>
                        <span className="flex gap-1 text-xs text-muted-foreground">
                          <span className="shrink-0">{formatDate(row.date)}</span>
                          {row.location && (
                            <span data-private className="truncate">
                              · {row.location}
                            </span>
                          )}
                        </span>
                      </span>
                      <span className="shrink-0 text-right text-xs text-muted-foreground tabular-nums">
                        {formatNumber(row.distance, 2)} km
                        <br />
                        {formatDuration(row.duration)}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
        <DialogFooter className="items-center sm:justify-between">
          <span className="text-sm text-muted-foreground">
            {t("{{count}} of {{max}} selected", { count: selected.length, max: MAX_COMPARE })}
          </span>
          <Button onClick={() => onOpenChange(false)}>{t("Done")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
