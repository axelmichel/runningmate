import {
  ArrowDown,
  ArrowUp,
  ChevronLeft,
  ChevronRight,
  EllipsisVertical,
  Eye,
  RotateCcw,
  Trash2,
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
import { Checkbox } from "@/components/ui/checkbox";
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
import { formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { columnsFor } from "./columns";

export type Sort = { field: SortField; direction: SortDirection };

/** Width of the checkbox column in px. */
const CHECKBOX_WIDTH = 40;
/** Data columns pinned next to the checkbox while the table scrolls sideways. */
const PINNED_COLUMNS = 2;

/** Opaque backgrounds for pinned cells, matching the row's hover and selected states. */
const PINNED_CELL =
  "sticky z-[1] bg-background group-hover/row:bg-[color-mix(in_oklab,var(--color-muted)_50%,var(--color-background))] group-data-[state=selected]/row:bg-muted";
const PINNED_HEAD = "sticky z-[2] bg-background";

type Pin = { left: number; width: number; last: boolean };

/** Left offsets of the checkbox and pinned data columns. */
function pinsFor(widths: (number | undefined)[]): (Pin | undefined)[] {
  const pins: (Pin | undefined)[] = [{ left: 0, width: CHECKBOX_WIDTH, last: false }];
  let left = CHECKBOX_WIDTH;
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
  /** Rows ticked for the distance total. */
  selection: ReadonlySet<number>;
  onSelectionChange: (selection: Set<number>) => void;
  onDelete: (row: ActivityRow) => void;
  /** Recompute from the archived TCX file. */
  onReimport: (row: ActivityRow) => void;
  onOpenDetails: (row: ActivityRow) => void;
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
  selection,
  onSelectionChange,
  onDelete,
  onReimport,
  onOpenDetails,
}: ActivityTableProps) {
  const [pendingDelete, setPendingDelete] = useState<ActivityRow | null>(null);
  const columns = columnsFor(view);
  // Index 0 is the checkbox column, then one entry per data column.
  const pins = pinsFor(columns.map((column) => column.width));
  const pinClass = (pin: Pin | undefined, base: string) =>
    pin && cn(base, pin.last && "shadow-[inset_-1px_0_0_var(--color-border)]");
  const rows = page?.rows ?? [];
  const total = page?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const selectedDistance = rows
    .filter((row) => selection.has(row.id))
    .reduce((sum, row) => sum + (row.distance ?? 0), 0);

  const toggle = (id: number, checked: boolean) => {
    const next = new Set(selection);
    if (checked) next.add(id);
    else next.delete(id);
    onSelectionChange(next);
  };

  // The header checkbox covers the rows on the current page only.
  const selectedOnPage = rows.filter((row) => selection.has(row.id)).length;
  const pageState =
    selectedOnPage === 0
      ? false
      : selectedOnPage === rows.length
        ? true
        : ("indeterminate" as const);
  const togglePage = () => {
    const next = new Set(selection);
    for (const row of rows) {
      if (pageState === true) next.delete(row.id);
      else next.add(row.id);
    }
    onSelectionChange(next);
  };

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
              <TableHead className={pinClass(pins[0], PINNED_HEAD)} style={pinStyle(pins[0])}>
                <Checkbox
                  aria-label={
                    pageState === true ? "Deselect all on this page" : "Select all on this page"
                  }
                  title={
                    pageState === true ? "Deselect all on this page" : "Select all on this page"
                  }
                  disabled={rows.length === 0}
                  checked={pageState}
                  onCheckedChange={togglePage}
                />
              </TableHead>
              {columns.map((column, index) => (
                <TableHead
                  key={column.sortField}
                  style={pinStyle(pins[index + 1])}
                  className={cn(
                    column.align === "right" && "text-right",
                    pinClass(pins[index + 1], PINNED_HEAD),
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
                    {column.header}
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
                <TableCell
                  className={pinClass(pins[0], PINNED_CELL)}
                  style={pinStyle(pins[0])}
                  onClick={(event) => event.stopPropagation()}
                >
                  <Checkbox
                    aria-label="Include in distance total"
                    checked={selection.has(row.id)}
                    onCheckedChange={(checked) => toggle(row.id, checked === true)}
                  />
                </TableCell>
                {columns.map((column, index) => (
                  <TableCell
                    key={column.sortField}
                    style={pinStyle(pins[index + 1])}
                    className={cn(
                      column.align === "right" && "text-right tabular-nums",
                      pinClass(pins[index + 1], PINNED_CELL),
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
                        aria-label="Activity actions"
                        className="text-muted-foreground"
                      >
                        <EllipsisVertical />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onSelect={() => onOpenDetails(row)}>
                        <Eye />
                        View details
                      </DropdownMenuItem>
                      <DropdownMenuItem onSelect={() => onReimport(row)}>
                        <RotateCcw />
                        Re-import from TCX
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        variant="destructive"
                        onSelect={() => setPendingDelete(row)}
                      >
                        <Trash2 />
                        Delete…
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </TableCell>
              </TableRow>
            ))}
            {page && rows.length === 0 && (
              <TableRow>
                <TableCell
                  colSpan={columns.length + 2}
                  className="py-10 text-center text-muted-foreground"
                >
                  No activities match.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      <div className="flex items-center justify-between gap-4 pt-3 text-sm text-muted-foreground">
        <span>
          {selection.size > 0 &&
            `${selection.size} selected · ${formatNumber(selectedDistance, 2)} km`}
        </span>
        <div className="flex items-center gap-2">
          <span className="tabular-nums">
            Page {pageIndex + 1} / {pageCount} · {total === 0 ? 0 : pageIndex * pageSize + 1}–
            {Math.min(total, (pageIndex + 1) * pageSize)} of {total}
          </span>
          <Button
            variant="outline"
            size="icon-sm"
            aria-label="Previous page"
            disabled={pageIndex === 0}
            onClick={() => onPageChange(pageIndex - 1)}
          >
            <ChevronLeft />
          </Button>
          <Button
            variant="outline"
            size="icon-sm"
            aria-label="Next page"
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
            <AlertDialogTitle>Delete activity?</AlertDialogTitle>
            <AlertDialogDescription>
              “{pendingDelete?.title}” and its segments, weather and best performances will be
              removed. Photos and videos stay in the media folder.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => pendingDelete && onDelete(pendingDelete)}
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
