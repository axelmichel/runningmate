import type { LucideIcon } from "lucide-react";

import { TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

export type NavItem = { value: string; label: string; icon: LucideIcon; disabled?: boolean };

/**
 * Vertical tab list styled like the app sidebar, for use inside a vertical `Tabs`. Collapses to
 * icons when `open` is false; labels then show as tooltips.
 */
export function NavTabs({ items, open = true }: { items: NavItem[]; open?: boolean }) {
  return (
    <TabsList
      className={cn(
        "h-fit shrink-0 flex-col items-stretch gap-0.5 overflow-hidden bg-transparent p-0 transition-[width] duration-200 ease-linear",
        open ? "w-48" : "w-10",
      )}
    >
      {items.map(({ value, label, icon: Icon, disabled }) => {
        const trigger = (
          <TabsTrigger
            key={value}
            value={value}
            disabled={disabled}
            aria-label={label}
            className={cn(
              "h-8 justify-start gap-2 overflow-hidden border-transparent px-2.5 hover:bg-muted/60",
              // Set for both themes: the trigger's own dark-mode active style is nearly invisible
              // without the tab list background.
              "data-active:bg-muted data-active:text-foreground data-active:shadow-none data-active:[&_svg]:text-primary",
              "dark:data-active:border-transparent dark:data-active:bg-muted dark:data-active:text-foreground",
            )}
          >
            <Icon className="shrink-0" />
            <span className={cn("truncate", !open && "sr-only")}>{label}</span>
          </TabsTrigger>
        );
        return open ? (
          trigger
        ) : (
          <Tooltip key={value}>
            <TooltipTrigger asChild>{trigger}</TooltipTrigger>
            <TooltipContent side="right">{label}</TooltipContent>
          </Tooltip>
        );
      })}
    </TabsList>
  );
}
