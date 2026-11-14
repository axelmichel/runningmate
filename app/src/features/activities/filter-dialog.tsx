import { useState, type ReactNode } from "react";

import type { ActivityFilter } from "@/bindings/ActivityFilter";
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
import { Label } from "@/components/ui/label";

import { draftToFilter, toDraft, type Draft } from "./filters";

type FilterDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  filter: ActivityFilter;
  onApply: (filter: ActivityFilter) => void;
};

export function FilterDialog({ open, onOpenChange, filter, onApply }: FilterDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {/* Keyed so the form restarts from the current filter each time it opens. */}
        {open && (
          <FilterForm
            filter={filter}
            onCancel={() => onOpenChange(false)}
            onApply={(next) => {
              onApply(next);
              onOpenChange(false);
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

function FilterForm({
  filter,
  onCancel,
  onApply,
}: {
  filter: ActivityFilter;
  onCancel: () => void;
  onApply: (filter: ActivityFilter) => void;
}) {
  const [draft, setDraft] = useState<Draft>(() => toDraft(filter));
  const field = (key: keyof Draft, type: "date" | "number") => (
    <Input
      id={key}
      type={type}
      min={type === "number" ? 0 : undefined}
      step="any"
      value={draft[key]}
      onChange={(event) => setDraft({ ...draft, [key]: event.target.value })}
    />
  );
  const row = (label: string, unit: string, min: ReactNode, max: ReactNode, id: keyof Draft) => (
    <div className="grid grid-cols-[7rem_1fr_1fr] items-center gap-2">
      <Label htmlFor={id}>
        {label}
        {unit && <span className="text-muted-foreground"> ({unit})</span>}
      </Label>
      {min}
      {max}
    </div>
  );

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        onApply(draftToFilter(draft, filter.searchText));
      }}
    >
      <DialogHeader>
        <DialogTitle>Filter activities</DialogTitle>
        <DialogDescription>Leave a field empty to not filter by it.</DialogDescription>
      </DialogHeader>
      <div className="grid gap-3">
        <div className="grid grid-cols-[7rem_1fr_1fr] gap-2 text-xs text-muted-foreground">
          <span />
          <span>From</span>
          <span>To</span>
        </div>
        {row("Date", "", field("minDate", "date"), field("maxDate", "date"), "minDate")}
        {row(
          "Distance",
          "km",
          field("minDistance", "number"),
          field("maxDistance", "number"),
          "minDistance",
        )}
        {row(
          "Duration",
          "min",
          field("minDuration", "number"),
          field("maxDuration", "number"),
          "minDuration",
        )}
        {row(
          "Elevation",
          "m",
          field("minElevation", "number"),
          field("maxElevation", "number"),
          "minElevation",
        )}
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit">Apply</Button>
      </DialogFooter>
    </form>
  );
}
