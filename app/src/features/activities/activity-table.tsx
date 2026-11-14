import { ArrowDown, ArrowUp, ChevronLeft, ChevronRight, Trash2 } from "lucide-react";
import { useState } from "react";

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
}: ActivityTableProps) {
  const [pendingDelete, setPendingDelete] = useState<ActivityRow | null>(null);
  const columns = columnsFor(view);
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

  const toggleSort = (field: SortField) =>
    onSortChange(
      sort.field === field
        ? { field, direction: sort.direction === "asc" ? "desc" : "asc" }
        : { field, direction: field === "title" || field === "type" ? "asc" : "desc" },
    );

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="min-h-0 flex-1 overflow-auto rounded-lg border">
        <Table>
          <TableHeader className="sticky top-0 z-10 bg-background">
            <TableRow>
              <TableHead className="w-8" />
              {columns.map((column) => (
                <TableHead
                  key={column.sortField}
                  className={cn(column.align === "right" && "text-right")}
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
                className="cursor-pointer"
                onClick={() => onActivate(row.id)}
              >
                <TableCell onClick={(event) => event.stopPropagation()}>
                  <Checkbox
                    aria-label="Include in distance total"
                    checked={selection.has(row.id)}
                    onCheckedChange={(checked) => toggle(row.id, checked === true)}
                  />
                </TableCell>
                {columns.map((column) => (
                  <TableCell
                    key={column.sortField}
                    className={cn(column.align === "right" && "text-right tabular-nums")}
                  >
                    {column.render(row)}
                  </TableCell>
                ))}
                <TableCell onClick={(event) => event.stopPropagation()}>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="Delete activity"
                    className="text-muted-foreground hover:text-destructive"
                    onClick={() => setPendingDelete(row)}
                  >
                    <Trash2 />
                  </Button>
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
