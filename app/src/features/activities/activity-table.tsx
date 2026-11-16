import {
  ArrowDown,
  ArrowUp,
  ChevronLeft,
  ChevronRight,
  EllipsisVertical,
  Eye,
  RotateCcw,
  Trash2,
  Upload,
} from "lucide-react";
import { useState, type CSSProperties } from "react";

import type { ActivityPage } from "@/bindings/ActivityPage";
import type { ActivityRow } from "@/bindings/ActivityRow";
import type { SortDirection } from "@/bindings/SortDirection";
import type { SortField } from "@/bindings/SortField";
import type { ViewMode } from "@/bindings/ViewMode";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { t } from "@/i18n";
import { cn } from "@/lib/utils";

import { columnsFor } from "./columns";

export type Sort = { field: SortField; direction: SortDirection };

/** Data columns pinned on the left while the table scrolls sideways. */
const PINNED_COLUMNS = 2;

/** Opaque backgrounds for pinned cells, matching the row's hover and selected states. */
const PINNED_CELL =
  "sticky z-[1] bg-background group-hover/row:bg-[color-mix(in_oklab,var(--color-muted)_50%,var(--color-background))] group-data-[state=selected]/row:bg-muted";
const PINNED_HEAD = "sticky z-[2] bg-background";

type Pin = { left: number; width: number; last: boolean };

/** Left offsets of the pinned data columns. */
function pinsFor(widths: (number | undefined)[]): (Pin | undefined)[] {
  const pins: (Pin | undefined)[] = [];
  let left = 0;
  widths.forEach((width, index) => {
    if (index >= PINNED_COLUMNS || width === undefined) {
      pins.push(undefined);
      return;
    }
    pins.push({ left, width, last: index === PINNED_COLUMNS - 1 });
    left += width;
  });
  return pins;
}

function pinStyle(pin: Pin | undefined): CSSProperties | undefined {
  return pin && { left: pin.left, width: pin.width, minWidth: pin.width, maxWidth: pin.width };
}

type ActivityTableProps = {
  view: ViewMode;
  page: ActivityPage | undefined;
  pageIndex: number;
  pageSize: number;
  onPageChange: (pageIndex: number) => void;
  sort: Sort;
  onSortChange: (sort: Sort) => void;
  /** Activity shown in the side panel. */
  activeId: number | null;
  onActivate: (id: number) => void;
  onDelete: (row: ActivityRow) => void;
  /** Recompute from the archived TCX file. */
  onReimport: (row: ActivityRow) => void;
  onOpenDetails: (row: ActivityRow) => void;
  onUploadGarmin: (row: ActivityRow) => void;
};

export function ActivityTable({
  view,
  page,
  pageIndex,
  pageSize,
  onPageChange,
  sort,
  onSortChange,
  activeId,
  onActivate,
  onDelete,
  onReimport,
  onOpenDetails,
  onUploadGarmin,
}: ActivityTableProps) {
  const [pendingDelete, setPendingDelete] = useState<ActivityRow | null>(null);
  const columns = columnsFor(view);
  // One entry per data column.
  const pins = pinsFor(columns.map((column) => column.width));
  const pinClass = (pin: Pin | undefined, base: string) =>
    pin && cn(base, pin.last && "shadow-[inset_-1px_0_0_var(--color-border)]");
  const rows = page?.rows ?? [];
  const total = page?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));

  const toggleSort = (field: SortField) =>
    onSortChange(
      sort.field === field
        ? { field, direction: sort.direction === "asc" ? "desc" : "asc" }
        : { field, direction: field === "title" || field === "type" ? "asc" : "desc" },
    );

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border">
        {/* The table container is the only scroll area, so the header and the pinned columns
            stay in place in both directions. */}
        <Table containerClassName="scrollbar-visible min-h-0 flex-1 overflow-auto">
          <TableHeader className="sticky top-0 z-10 bg-background">
            <TableRow>
              {columns.map((column, index) => (
                <TableHead
                  key={column.sortField}
                  style={pinStyle(pins[index])}
                  className={cn(
                    column.align === "right" && "text-right",
                    pinClass(pins[index], PINNED_HEAD),
                  )}
                  aria-sort={
                    sort.field === column.sortField
                      ? sort.direction === "asc"
                        ? "ascending"
                        : "descending"
                      : undefined
                  }
                >
                  <button
                    type="button"
                    className={cn(
                      "inline-flex items-center gap-1 hover:text-foreground",
                      sort.field === column.sortField && "text-foreground",
                    )}
                    onClick={() => toggleSort(column.sortField)}
                  >
                    {t(column.header)}
                    {sort.field === column.sortField &&
                      (sort.direction === "asc" ? (
                        <ArrowUp className="size-3" />
                      ) : (
                        <ArrowDown className="size-3" />
                      ))}
                  </button>
                </TableHead>
              ))}
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow
                key={row.id}
                data-state={row.id === activeId ? "selected" : undefined}
                className="group/row cursor-pointer"
                onClick={() => onActivate(row.id)}
                onDoubleClick={() => onOpenDetails(row)}
              >
                {columns.map((column, index) => (
                  <TableCell
                    key={column.sortField}
                    style={pinStyle(pins[index])}
                    className={cn(
                      column.align === "right" && "text-right tabular-nums",
                      pinClass(pins[index], PINNED_CELL),
                    )}
                  >
                    {column.render(row)}
                  </TableCell>
                ))}
                <TableCell onClick={(event) => event.stopPropagation()}>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={t("Activity actions")}
                        className="text-muted-foreground"
                      >
                        <EllipsisVertical />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onSelect={() => onOpenDetails(row)}>
                        <Eye />
                        {t("View details")}
                      </DropdownMenuItem>
                      <DropdownMenuItem onSelect={() => onReimport(row)}>
                        <RotateCcw />
                        {t("Re-import from TCX")}
                      </DropdownMenuItem>
                      <DropdownMenuItem onSelect={() => onUploadGarmin(row)}>
                        <Upload />
                        {t("Upload to Garmin Connect")}
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        variant="destructive"
                        onSelect={() => setPendingDelete(row)}
                      >
                        <Trash2 />
                        {t("Delete…")}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </TableCell>
              </TableRow>
            ))}
            {page && rows.length === 0 && (
              <TableRow>
                <TableCell
                  colSpan={columns.length + 1}
                  className="py-10 text-center text-muted-foreground"
                >
                  {t("No activities match.")}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      <div className="flex items-center justify-end gap-4 pt-3 text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <span className="tabular-nums">
            {t("Page {{page}} / {{pages}} · {{from}}–{{to}} of {{total}}", {
              page: pageIndex + 1,
              pages: pageCount,
              from: total === 0 ? 0 : pageIndex * pageSize + 1,
              to: Math.min(total, (pageIndex + 1) * pageSize),
              total,
            })}
          </span>
          <Button
            variant="outline"
            size="icon-sm"
            aria-label={t("Previous page")}
            disabled={pageIndex === 0}
            onClick={() => onPageChange(pageIndex - 1)}
          >
            <ChevronLeft />
          </Button>
          <Button
            variant="outline"
            size="icon-sm"
            aria-label={t("Next page")}
            disabled={pageIndex + 1 >= pageCount}
            onClick={() => onPageChange(pageIndex + 1)}
          >
            <ChevronRight />
          </Button>
        </div>
      </div>

      <AlertDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Delete activity?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                "“{{title}}” and its segments, weather and best performances will be removed. Photos and videos stay in the media folder.",
                { title: pendingDelete?.title },
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => pendingDelete && onDelete(pendingDelete)}
            >
              {t("Delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
