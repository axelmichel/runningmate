import { Filter, Search, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { ActivityFilter } from "@/bindings/ActivityFilter";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

import { FilterDialog } from "./filter-dialog";
import { filterLabels } from "./filters";

type SearchBarProps = {
  filter: ActivityFilter;
  onFilterChange: (filter: ActivityFilter) => void;
  onClose: () => void;
};

export function SearchBar({ filter, onFilterChange, onClose }: SearchBarProps) {
  const [text, setText] = useState(filter.searchText ?? "");
  const [dialogOpen, setDialogOpen] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const labels = filterLabels(filter);
  const active = Boolean(filter.searchText) || labels.length > 0;

  useEffect(() => input.current?.focus(), []);

  // Search as you type, after a short pause.
  useEffect(() => {
    const trimmed = text.trim();
    if (trimmed === (filter.searchText ?? "")) return;
    const timer = setTimeout(
      () => onFilterChange({ ...filter, searchText: trimmed || undefined }),
      250,
    );
    return () => clearTimeout(timer);
  }, [text, filter, onFilterChange]);

  return (
    <div className="flex flex-wrap items-center gap-2">
      <div className="relative w-72">
        <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          ref={input}
          value={text}
          placeholder="Search title or comment…"
          className="pl-8"
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => event.key === "Escape" && onClose()}
        />
      </div>
      <Button variant="outline" size="sm" onClick={() => setDialogOpen(true)}>
        <Filter />
        Filter
      </Button>
      {labels.map((label) => (
        <Badge key={label} variant="secondary">
          {label}
        </Badge>
      ))}
      {active && (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setText("");
            onFilterChange({});
          }}
        >
          <X />
          Reset
        </Button>
      )}
      <FilterDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        filter={filter}
        onApply={onFilterChange}
      />
    </div>
  );
}
